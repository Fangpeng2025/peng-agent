//! 上下文压缩器
//!
//! 当对话历史过长时，自动压缩中间消息以保持在 token 预算内。
//! 保留策略：系统提示词（第一条）+ 最近 N 条消息 + 压缩后的中间摘要。

use crate::config::KernelConfig;
use crate::llm::LlmClient;
use crate::types::{ChatMessage, Result};

// ============================================================================
// 上下文压缩器
// ============================================================================

/// 上下文压缩器
///
/// 管理对话历史的长度，当总字符数超过阈值时自动触发压缩。
/// 压缩策略：保留系统消息 + 最近 N 条消息，将中间旧消息压缩为摘要。
pub struct ContextCompressor {
    /// 是否启用压缩
    enabled: bool,
    /// 字符数阈值（超过此值触发压缩）
    threshold: usize,
    /// LLM 客户端（可选，用于 LLM 压缩）
    llm: Option<LlmClient>,
}

impl ContextCompressor {
    /// 创建上下文压缩器
    ///
    /// # 参数
    /// - `config`: 内核配置（读取 compression_enabled 和 compression_threshold）
    /// - `llm`: LLM 客户端（可选，提供时使用 LLM 压缩，否则使用滑动窗口）
    ///
    /// # 返回
    /// 初始化成功返回 ContextCompressor
    pub fn new(config: &KernelConfig, llm: Option<LlmClient>) -> Result<Self> {
        Ok(Self {
            enabled: config.compression_enabled,
            threshold: config.compression_threshold as usize,
            llm,
        })
    }

    /// 检查是否需要压缩
    ///
    /// 计算所有消息内容的总字符数，判断是否超过阈值。
    ///
    /// # 参数
    /// - `messages`: 对话消息列表
    ///
    /// # 返回
    /// 超过阈值且压缩已启用时返回 true
    pub fn should_compress(&self, messages: &[ChatMessage]) -> bool {
        if !self.enabled {
            return false;
        }

        let total_chars: usize = messages
            .iter()
            .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
            .sum();

        total_chars > self.threshold
    }

    /// 压缩消息列表
    ///
    /// 压缩策略：
    /// 1. 始终保留第一条消息（系统提示词）
    /// 2. 始终保留最后 `keep_recent` 条消息
    /// 3. 中间消息压缩为一条摘要消息
    ///
    /// 如果 LLM 客户端可用，使用 LLM 生成摘要；
    /// 否则使用简单的文本截断。
    ///
    /// # 参数
    /// - `messages`: 原始消息列表
    /// - `keep_recent`: 保留最近的消息条数
    ///
    /// # 返回
    /// 压缩后的消息列表
    pub fn compress_messages(
        &self,
        messages: &[ChatMessage],
        keep_recent: usize,
    ) -> Result<Vec<ChatMessage>> {
        if messages.len() <= keep_recent + 1 {
            // 消息太少，不需要压缩
            return Ok(messages.to_vec());
        }

        let mut result = Vec::new();

        // 1. 保留系统提示词（第一条消息，如果是 system 角色的话）
        let system_msg = if !messages.is_empty() && messages[0].role == "system" {
            Some(messages[0].clone())
        } else {
            None
        };

        // 2. 分离中间消息和最近消息
        let start_idx = if system_msg.is_some() { 1 } else { 0 };
        let recent_start = messages.len().saturating_sub(keep_recent);

        let middle_messages: &[ChatMessage] = if recent_start > start_idx {
            &messages[start_idx..recent_start]
        } else {
            &[]
        };

        let recent_messages: &[ChatMessage] = &messages[recent_start..];

        // 3. 压缩中间消息
        if let Some(ref sys) = system_msg {
            result.push(sys.clone());
        }

        if !middle_messages.is_empty() {
            let summary = self.compress_middle_messages(middle_messages)?;
            result.push(ChatMessage::system(&summary));
        }

        // 4. 添加最近消息
        result.extend(recent_messages.iter().cloned());

        log::info!(
            "上下文压缩: {} 条 → {} 条",
            messages.len(),
            result.len()
        );

        Ok(result)
    }

    /// 压缩中间消息为摘要
    ///
    /// 如果 LLM 客户端可用，使用 LLM 生成摘要；
    /// 否则将所有消息拼接后截断。
    fn compress_middle_messages(&self, messages: &[ChatMessage]) -> Result<String> {
        if self.llm.is_some() {
            // 使用 LLM 压缩
            let text = self.messages_to_text(messages);
            // 在同步上下文中无法直接调用 async LLM，这里使用简单截断
            // TODO: 在 async 上下文中使用 LLM 压缩
            log::info!("LLM 压缩暂未实现，使用滑动窗口截断");
            Ok(self.truncate_text(&text))
        } else {
            // 简单拼接 + 截断
            let text = self.messages_to_text(messages);
            Ok(self.truncate_text(&text))
        }
    }

    /// 异步版本的中间消息压缩（使用 LLM）
    ///
    /// # 参数
    /// - `messages`: 需要压缩的中间消息列表
    ///
    /// # 返回
    /// 压缩后的摘要文本
    pub async fn compress_middle_messages_async(&self, messages: &[ChatMessage]) -> Result<String> {
        if let Some(ref llm) = self.llm {
            let text = self.messages_to_text(messages);
            llm.compress("对话历史压缩", &text).await
        } else {
            let text = self.messages_to_text(messages);
            Ok(self.truncate_text(&text))
        }
    }

    /// 将消息列表转换为文本
    fn messages_to_text(&self, messages: &[ChatMessage]) -> String {
        let mut text = String::new();
        for msg in messages {
            if let Some(ref content) = msg.content {
                let role_label = match msg.role.as_str() {
                    "user" => "用户",
                    "assistant" => "助手",
                    "system" => "系统",
                    "tool" => "工具",
                    _ => "未知",
                };
                text.push_str(&format!("[{role_label}] {content}\n"));
            }
        }
        text
    }

    /// 截断过长的文本
    ///
    /// 保留前 half/2 和后 half/2 字符，中间用省略号连接。
    fn truncate_text(&self, text: &str) -> String {
        let max_len = self.threshold / 2; // 摘要最多占阈值的一半
        if text.len() <= max_len {
            return format!("=== 对话历史摘要 ===\n{text}=== 摘要结束 ===");
        }

        let half = max_len / 2;
        let front: String = text.chars().take(half).collect();
        let back: String = text.chars().skip(text.chars().count() - half).collect();

        format!(
            "=== 对话历史摘要 ===\n{front}\n\n... (省略 {} 字符) ...\n\n{back}\n=== 摘要结束 ===",
            text.len() - half * 2
        )
    }

    /// 估算文本的 token 数
    ///
    /// 粗略估算：中文约 2 字符/token，英文约 4 字符/token。
    /// 取折中值：3 字符/token。
    ///
    /// # 参数
    /// - `text`: 要估算的文本
    ///
    /// # 返回
    /// 估算的 token 数
    pub fn estimate_tokens(text: &str) -> usize {
        // 统计中文字符数
        let chinese_count = text
            .chars()
            .filter(|c| '\u{4e00}' <= *c && *c <= '\u{9fff}')
            .count();

        let non_chinese_len = text.len() - chinese_count * 3; // UTF-8 中文约3字节

        // 中文约 1.5 字符/token，英文约 4 字符/token
        let chinese_tokens = (chinese_count as f64 / 1.5) as usize;
        let english_tokens = non_chinese_len / 4;

        chinese_tokens + english_tokens
    }

    /// 压缩单段文本
    ///
    /// 如果文本超过阈值，截断中间部分。
    ///
    /// # 参数
    /// - `text`: 要压缩的文本
    ///
    /// # 返回
    /// 压缩后的文本
    pub fn compress_text(&self, text: &str) -> Result<String> {
        if text.len() <= self.threshold {
            return Ok(text.to_string());
        }
        Ok(self.truncate_text(text))
    }

    /// 获取压缩阈值
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "test", "api_key": "sk-test", "compression_enabled": true, "compression_threshold": 100}"#,
        )
        .unwrap()
    }

    #[test]
    fn test_should_compress_below_threshold() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let messages = vec![
            ChatMessage::system("你是助手"),
            ChatMessage::user("你好"),
        ];

        assert!(!compressor.should_compress(&messages));
    }

    #[test]
    fn test_should_compress_above_threshold() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let long_content: String = "这是一段很长的文本。".repeat(20); // ~200 chars
        let messages = vec![
            ChatMessage::system("你是助手"),
            ChatMessage::user(&long_content),
        ];

        assert!(compressor.should_compress(&messages));
    }

    #[test]
    fn test_should_compress_disabled() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "test", "api_key": "sk-test", "compression_enabled": false, "compression_threshold": 10}"#,
        )
        .unwrap();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let long_content: String = "很长的文本。".repeat(100);
        let messages = vec![ChatMessage::user(&long_content)];

        assert!(!compressor.should_compress(&messages));
    }

    #[test]
    fn test_compress_messages_keep_recent() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let messages = vec![
            ChatMessage::system("系统提示"),
            ChatMessage::user("消息1"),
            ChatMessage::assistant("回复1"),
            ChatMessage::user("消息2"),
            ChatMessage::assistant("回复2"),
            ChatMessage::user("消息3"),
            ChatMessage::assistant("回复3"),
        ];

        let compressed = compressor.compress_messages(&messages, 2).unwrap();

        // 应保留：system + summary + last 2 = at least 4
        assert!(compressed.len() < messages.len());
        assert_eq!(compressed[0].role, "system");
        // 最后两条应该是原始消息
        assert_eq!(compressed.last().unwrap().content.as_deref(), Some("回复3"));
    }

    #[test]
    fn test_compress_messages_too_few() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let messages = vec![
            ChatMessage::system("系统提示"),
            ChatMessage::user("你好"),
        ];

        let compressed = compressor.compress_messages(&messages, 5).unwrap();
        // 消息太少，不需要压缩
        assert_eq!(compressed.len(), messages.len());
    }

    #[test]
    fn test_estimate_tokens() {
        let english = "Hello, how are you today?";
        let tokens = ContextCompressor::estimate_tokens(english);
        assert!(tokens > 0);
        assert!(tokens < english.len()); // 应该比字符数少

        let chinese = "你好，今天天气怎么样？";
        let cn_tokens = ContextCompressor::estimate_tokens(chinese);
        assert!(cn_tokens > 0);
    }

    #[test]
    fn test_truncate_text() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let short_text = "短文本";
        let result = compressor.truncate_text(short_text);
        assert!(result.contains("对话历史摘要"));
        assert!(result.contains("短文本"));

        let long_text: String = "这是一段很长的文本用于测试截断功能。".repeat(20);
        let result = compressor.truncate_text(&long_text);
        assert!(result.contains("省略"));
    }

    #[test]
    fn test_compress_text_short() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let short = "短文本不需要压缩";
        let result = compressor.compress_text(short).unwrap();
        assert_eq!(result, short);
    }

    #[test]
    fn test_compress_text_long() {
        let config = test_config();
        let compressor = ContextCompressor::new(&config, None).unwrap();

        let long: String = "很长的文本需要压缩。".repeat(20);
        let result = compressor.compress_text(&long).unwrap();
        assert!(result.len() < long.len());
    }
}
