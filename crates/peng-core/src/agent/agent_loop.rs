//! 智能体主循环 — Agent Loop
//!
//! 这是鹏智能体的核心循环，负责：
//! 1. 接收用户消息与历史对话
//! 2. 注入系统提示词（含技能、记忆上下文）
//! 3. 调用 LLM 流式接口（带工具支持）
//! 4. 若 LLM 返回 tool_calls → 逐个执行工具 → 将结果回传 → 继续循环
//! 5. 若 LLM 返回纯文本 → 完成，返回响应
//! 6. 遵守 max_tool_rounds 限制
//! 7. 支持通过 AtomicBool 中止

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::config::KernelConfig;
use crate::llm::LlmClient;
use crate::tools::ToolRouter;
use crate::types::*;

// ============================================================================
// Agent Loop
// ============================================================================

/// 智能体主循环
///
/// 管理完整的对话循环：系统提示词注入 → LLM 调用 → 工具执行 → 结果回传 → 循环。
/// 支持流式输出回调、工具调用路由和中止控制。
pub struct AgentLoop {
    /// 内核配置
    config: KernelConfig,
    /// LLM 客户端
    llm: LlmClient,
    /// 工具路由器（来自 crate::tools 模块）
    tool_router: ToolRouter,
    /// 中止标志
    abort_flag: Arc<AtomicBool>,
    /// 会话标识
    session_id: String,
}

impl AgentLoop {
    /// 创建新的智能体主循环
    ///
    /// 从 [`KernelConfig`] 初始化 LLM 客户端和工具路由器。
    ///
    /// # 参数
    /// - `config`: 内核配置
    ///
    /// # 返回
    /// 初始化成功返回 AgentLoop，失败返回 Error
    pub fn new(config: KernelConfig) -> Result<Self> {
        let llm = LlmClient::new(&config)?;
        let tool_router = ToolRouter::new(&config)?;
        let session_id = format!("session_{}", uuid_short());

        log::info!(
            "AgentLoop 初始化: model={}, session={}",
            config.model,
            session_id
        );

        Ok(Self {
            config,
            llm,
            tool_router,
            abort_flag: Arc::new(AtomicBool::new(false)),
            session_id,
        })
    }

    /// 运行智能体主循环 — THE MAIN METHOD
    ///
    /// 执行完整的对话循环：
    /// 1. 构建系统提示词（含技能、记忆上下文）
    /// 2. 组装完整消息列表：[system] + history + [user_message]
    /// 3. 循环调用 LLM，最多 max_tool_rounds 轮
    /// 4. 若 LLM 返回纯文本 → 直接返回
    /// 5. 若 LLM 返回 tool_calls → 执行工具 → 回传结果 → 继续循环
    ///
    /// # 参数
    /// - `user_message`: 用户输入消息
    /// - `history`: 历史对话消息列表
    /// - `callback`: 流式输出回调
    ///
    /// # 返回
    /// 智能体的最终文本响应
    pub async fn run(
        &self,
        user_message: &str,
        history: &[ChatMessage],
        callback: &dyn StreamCallback,
    ) -> Result<String> {
        // 重置中止标志
        self.abort_flag.store(false, Ordering::SeqCst);

        // 1. 构建系统提示词
        let system_prompt = self.build_system_prompt();
        let system_msg = ChatMessage::system(&system_prompt);

        // 2. 组装完整消息列表
        let mut messages: Vec<ChatMessage> = Vec::with_capacity(2 + history.len());
        messages.push(system_msg);
        messages.extend_from_slice(history);
        messages.push(ChatMessage::user(user_message));

        // 3. 获取工具定义
        let tools = self.tool_router.get_tool_definitions();

        // 4. 主循环
        let max_rounds = self.config.max_tool_rounds;
        let mut accumulated_content = String::new();

        for round in 0..max_rounds {
            // 检查中止标志
            if self.abort_flag.load(Ordering::SeqCst) {
                let msg = "智能体已被用户中止";
                callback.on_error(msg);
                return Err(Error::Other(msg.to_string()));
            }

            log::debug!(
                "AgentLoop 第 {}/{} 轮: session={}",
                round + 1,
                max_rounds,
                self.session_id
            );

            // 调用 LLM（流式 + 工具支持）
            let response = self
                .llm
                .chat_with_tools_stream(&messages, &tools, callback)
                .await?;

            // 提取文本内容
            if let Some(ref content) = response.content {
                if !content.is_empty() {
                    accumulated_content = content.clone();
                }
            }

            // 检查是否有工具调用
            match response.tool_calls.clone() {
                Some(tool_calls) if !tool_calls.is_empty() => {
                    // 有工具调用 → 执行工具 → 回传结果 → 继续循环
                    log::info!(
                        "AgentLoop 收到 {} 个工具调用: session={}",
                        tool_calls.len(),
                        self.session_id
                    );

                    // 将助手消息（含工具调用）加入消息列表
                    messages.push(response);

                    // 逐个执行工具（顺序执行）
                    for tool_call in &tool_calls {
                        // 再次检查中止标志
                        if self.abort_flag.load(Ordering::SeqCst) {
                            let msg = "智能体已被用户中止";
                            callback.on_error(msg);
                            return Err(Error::Other(msg.to_string()));
                        }

                        let tool_name = &tool_call.function.name;
                        let tool_args = &tool_call.function.arguments;

                        // 通知回调：工具开始执行
                        callback.on_tool_start(tool_name, tool_args);

                        // 执行工具（ToolRouter::execute 是同步方法）
                        let result = self
                            .tool_router
                            .execute(tool_name, tool_args)
                            .unwrap_or_else(|e| format!("工具执行错误: {e}"));

                        log::debug!(
                            "工具执行完成: name={}, result_len={}",
                            tool_name,
                            result.len()
                        );

                        // 通知回调：工具执行完毕
                        callback.on_tool_end(tool_name, &result);

                        // 将工具结果加入消息列表
                        messages.push(ChatMessage::tool_result(&tool_call.id, &result));
                    }

                    // 继续下一轮循环
                    continue;
                }
                _ => {
                    // 无工具调用 → 循环结束
                    log::info!(
                        "AgentLoop 完成: session={}, rounds={}",
                        self.session_id,
                        round + 1
                    );
                    return Ok(accumulated_content);
                }
            }
        }

        // 超过最大工具轮数限制
        let warning = format!(
            "⚠️ 已达到最大工具调用轮数限制 ({max_rounds} 轮)，对话可能未完成。"
        );
        log::warn!("AgentLoop 超出最大轮数: session={}", self.session_id);
        callback.on_error(&warning);

        // 返回已累积的内容（可能为空）
        if accumulated_content.is_empty() {
            Ok(warning)
        } else {
            Ok(accumulated_content)
        }
    }

    /// 中止当前运行的智能体循环
    ///
    /// 设置中止标志，run() 方法在下一轮循环开始时检测到后会停止执行。
    pub fn abort(&self) {
        self.abort_flag.store(true, Ordering::SeqCst);
        log::info!("AgentLoop 收到中止请求: session={}", self.session_id);
    }

    /// 重置会话状态
    ///
    /// 清除中止标志，生成新的会话标识。
    pub fn reset_session(&mut self) {
        self.abort_flag.store(false, Ordering::SeqCst);
        self.session_id = format!("session_{}", uuid_short());
        log::info!("AgentLoop 会话已重置: session={}", self.session_id);
    }

    /// 获取当前内核状态
    ///
    /// # 返回
    /// 包含初始化状态、模型名称和状态消息的 [`KernelStatus`]
    pub fn get_status(&self) -> KernelStatus {
        KernelStatus::initialized(&self.config.model)
    }

    /// 直接调用工具
    ///
    /// 不经过 LLM 循环，直接通过 ToolRouter 执行指定工具。
    /// 用于 Kotlin 端的 PHONE 领域回调场景。
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    /// - `params`: 工具参数（JSON 字符串）
    ///
    /// # 返回
    /// 工具执行结果字符串
    pub async fn call_tool(&self, tool_name: &str, params: &str) -> Result<String> {
        self.tool_router.execute(tool_name, params)
    }

    /// 列出所有可用工具定义
    ///
    /// # 返回
    /// 工具定义列表（JSON 可序列化）
    pub fn list_tools(&self) -> Vec<crate::llm::ToolDefinition> {
        self.tool_router.get_tool_definitions()
    }

    /// 获取当前会话标识
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 构建系统提示词
    ///
    /// 包含角色描述、可用工具摘要和技能上下文。
    fn build_system_prompt(&self) -> String {
        let mut prompt = String::with_capacity(2048);

        // 角色描述
        prompt.push_str(
            "你是鹏（Peng），一个功能强大的 AI 智能体助手。\
             你能够通过调用各种工具来帮助用户完成任务。\n\n",
        );

        // 可用工具摘要
        prompt.push_str("## 可用工具\n\n");
        let tools = self.tool_router.get_tool_definitions();
        if tools.is_empty() {
            prompt.push_str("当前无可用工具。\n\n");
        } else {
            for tool in &tools {
                prompt.push_str(&format!(
                    "- **{}**: {}\n",
                    tool.function.name, tool.function.description
                ));
            }
            prompt.push('\n');
        }

        // 工具使用指引
        prompt.push_str(
            "## 工具使用指引\n\n\
             - 优先使用工具完成任务，而非猜测答案\n\
             - 每次只调用必要的工具\n\
             - 工具参数必须为合法 JSON 格式\n\
             - 如果工具返回错误，分析原因后重试或告知用户\n\n",
        );

        // 技能上下文（如果有技能目录配置）
        if !self.config.skills_dir.is_empty() {
            prompt.push_str(&format!(
                "## 技能目录\n\n技能定义文件位于: {}\n\n",
                self.config.skills_dir
            ));
        }

        // 记忆上下文占位（后续由记忆模块填充）
        prompt.push_str("## 记忆上下文\n\n（暂无记忆数据）\n");

        prompt
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 生成简短伪 UUID（用于会话标识和临时文件名）
///
/// 使用时间戳和计数器生成，不保证全局唯一，但足够用于标识。
fn uuid_short() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}{:04x}", ts, count % 0x10000)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_loop_new() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp/peng_test", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let agent = AgentLoop::new(config);
        assert!(agent.is_ok());

        let agent = agent.unwrap();
        assert!(!agent.session_id.is_empty());
        assert!(agent.session_id.starts_with("session_"));
    }

    #[test]
    fn test_agent_loop_status() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp/peng_test", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let agent = AgentLoop::new(config).unwrap();
        let status = agent.get_status();
        assert!(status.initialized);
        assert_eq!(status.model, "deepseek-chat");
    }

    #[test]
    fn test_agent_loop_abort() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp/peng_test", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let agent = AgentLoop::new(config).unwrap();
        assert!(!agent.abort_flag.load(Ordering::SeqCst));

        agent.abort();
        assert!(agent.abort_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_agent_loop_reset_session() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp/peng_test", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let mut agent = AgentLoop::new(config).unwrap();
        let old_session = agent.session_id().to_string();

        agent.abort();
        assert!(agent.abort_flag.load(Ordering::SeqCst));

        agent.reset_session();
        assert!(!agent.abort_flag.load(Ordering::SeqCst));
        assert_ne!(agent.session_id(), old_session);
    }

    #[test]
    fn test_build_system_prompt() {
        let config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp/peng_test", "model": "deepseek-chat", "api_key": "sk-test", "skills_dir": "/skills"}"#,
        )
        .unwrap();

        let agent = AgentLoop::new(config).unwrap();
        let prompt = agent.build_system_prompt();

        assert!(prompt.contains("鹏"));
        assert!(prompt.contains("可用工具"));
        assert!(prompt.contains("execute_shell"));
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("工具使用指引"));
        assert!(prompt.contains("/skills"));
        assert!(prompt.contains("记忆上下文"));
    }

    #[test]
    fn test_uuid_short() {
        let id1 = uuid_short();
        let id2 = uuid_short();
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        // 两次调用应产生不同结果
        assert_ne!(id1, id2);
    }
}
