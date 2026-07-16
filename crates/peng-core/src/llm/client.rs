//! LLM 客户端
//!
//! 封装 reqwest HTTP 客户端，提供流式/非流式聊天接口和上下文压缩。
//! 支持 OpenAI 兼容格式和 Anthropic 格式的 LLM API。

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::config::KernelConfig;
use crate::types::*;

use super::provider::{detect_provider, Provider};
use super::sse::{accumulate_tool_calls, SseParser};

// ============================================================================
// 工具定义结构
// ============================================================================

/// 工具定义（OpenAI 格式）
///
/// 用于向 LLM 声明可调用的工具。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 类型，固定为 "function"
    #[serde(rename = "type")]
    pub type_field: String,
    /// 函数描述
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    /// 创建函数工具定义
    ///
    /// # 参数
    /// - `name`: 函数名称
    /// - `description`: 函数功能描述
    /// - `parameters`: JSON Schema 格式的参数定义
    pub fn function(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            type_field: "function".to_string(),
            function: FunctionDefinition {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// 函数名称
    pub name: String,
    /// 函数功能描述
    pub description: String,
    /// JSON Schema 格式的参数定义
    pub parameters: serde_json::Value,
}

// ============================================================================
// LLM 配置
// ============================================================================

/// LLM 客户端配置
///
/// 从 [`KernelConfig`] 中提取的 LLM 相关配置项。
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// API 端点地址
    pub endpoint: String,
    /// API 密钥
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 最大生成 token 数
    pub max_tokens: u32,
    /// 采样温度
    pub temperature: f64,
    /// 核采样概率阈值
    pub top_p: f64,
    /// 流式响应超时秒数
    pub stream_timeout_secs: u64,
}

impl From<&KernelConfig> for LlmConfig {
    fn from(config: &KernelConfig) -> Self {
        Self {
            endpoint: config.api_base.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            top_p: config.top_p,
            stream_timeout_secs: config.stream_timeout_secs,
        }
    }
}

// ============================================================================
// LLM 客户端
// ============================================================================

/// LLM 客户端
///
/// 封装 reqwest HTTP 客户端，提供流式和非流式聊天接口。
/// 根据 API 端点自动检测提供商，适配请求格式和认证方式。
pub struct LlmClient {
    /// HTTP 客户端
    client: reqwest::Client,
    /// LLM 配置
    config: LlmConfig,
    /// 检测到的提供商
    provider: Provider,
}

/// 最大重试次数
const MAX_RETRIES: u32 = 3;

impl LlmClient {
    /// 创建新的 LLM 客户端
    ///
    /// 从 [`KernelConfig`] 中提取配置，自动检测提供商，
    /// 构建 reqwest 客户端（含超时设置）。
    ///
    /// # 参数
    /// - `config`: 内核配置
    ///
    /// # 返回
    /// 初始化成功返回 LlmClient，失败返回 Error
    pub fn new(config: &KernelConfig) -> Result<Self> {
        let llm_config = LlmConfig::from(config);
        let provider = detect_provider(&llm_config.endpoint);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(llm_config.stream_timeout_secs))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Llm(format!("创建 HTTP 客户端失败: {e}")))?;

        log::info!("LLM 客户端初始化: provider={provider}, model={}", llm_config.model);

        Ok(Self {
            client,
            config: llm_config,
            provider,
        })
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        // Content-Type
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        // 认证头
        let (auth_name, auth_value) = self.provider.auth_header(&self.config.api_key);
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(auth_name.as_bytes()),
            reqwest::header::HeaderValue::from_str(&auth_value),
        ) {
            headers.insert(name, value);
        }

        // Anthropic 版本头
        if let Some(version) = self.provider.anthropic_version() {
            if let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(b"anthropic-version"),
                reqwest::header::HeaderValue::from_str(version),
            ) {
                headers.insert(name, value);
            }
        }

        headers
    }

    /// 构建请求体（OpenAI 兼容格式）
    fn build_body(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
        stream: bool,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "stream": stream,
        });

        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
            }
        }

        body
    }

    /// 构建请求体（Anthropic 格式）
    fn build_anthropic_body(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
        stream: bool,
    ) -> serde_json::Value {
        // Anthropic 格式需要将 system 消息提取到顶层
        let mut system_prompt = String::new();
        let mut filtered_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                if let Some(ref content) = msg.content {
                    if !system_prompt.is_empty() {
                        system_prompt.push('\n');
                    }
                    system_prompt.push_str(content);
                }
            } else {
                filtered_messages.push(msg);
            }
        }

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": filtered_messages,
            "max_tokens": self.config.max_tokens,
            "stream": stream,
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::json!(system_prompt);
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                // Anthropic 的工具格式略有不同
                let anthropic_tools: Vec<serde_json::Value> = tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.function.name,
                            "description": t.function.description,
                            "input_schema": t.function.parameters,
                        })
                    })
                    .collect();
                body["tools"] = serde_json::json!(anthropic_tools);
            }
        }

        body
    }

    /// 流式聊天（带工具支持）
    ///
    /// THE MAIN METHOD：发送消息到 LLM，以流式方式接收响应。
    /// 支持 OpenAI 兼容格式和 Anthropic 格式。
    ///
    /// # 流程
    /// 1. 构建请求体（stream: true）
    /// 2. 发送 POST 请求
    /// 3. 读取 bytes_stream()
    /// 4. 将数据块送入 SseParser 解析
    /// 5. 对每个事件：extract_delta → callback.on_token()
    /// 6. 累积工具调用增量
    /// 7. 收到 [DONE] 时：返回最终 ChatMessage（含 content + tool_calls）
    /// 8. 错误处理：超时、连接错误，最多重试 3 次
    ///
    /// # 参数
    /// - `messages`: 对话消息列表
    /// - `tools`: 可用工具定义列表
    /// - `callback`: 流式回调
    ///
    /// # 返回
    /// LLM 的完整响应消息（包含文本内容和可能的工具调用）
    pub async fn chat_with_tools_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        callback: &dyn StreamCallback,
    ) -> Result<ChatMessage> {
        let mut last_error: Option<String> = None;

        for attempt in 0..MAX_RETRIES {
            match self
                .chat_with_tools_stream_inner(messages, tools, callback)
                .await
            {
                Ok(msg) => return Ok(msg),
                Err(e) => {
                    let err_str = e.to_string();
                    log::warn!(
                        "LLM 流式请求第 {}/{} 次失败: {err_str}",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    last_error = Some(err_str.clone());

                    // 不可重试的错误直接返回
                    if is_non_retryable(&e) {
                        callback.on_error(&err_str);
                        return Err(e);
                    }

                    // 如果还有重试机会，等待后继续
                    if attempt + 1 < MAX_RETRIES {
                        let delay = Duration::from_millis(500 * 2u64.pow(attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        let err = last_error.unwrap_or_else(|| "未知错误".to_string());
        callback.on_error(&err);
        Err(Error::Llm(format!("LLM 流式请求重试 {MAX_RETRIES} 次后仍失败: {err}")))
    }

    /// 流式聊天内部实现（单次请求）
    async fn chat_with_tools_stream_inner(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        callback: &dyn StreamCallback,
    ) -> Result<ChatMessage> {
        let url = self.provider.base_url(&self.config.endpoint);
        let headers = self.build_headers();

        let body = if self.provider == Provider::Anthropic {
            self.build_anthropic_body(messages, Some(tools), true)
        } else {
            self.build_body(messages, Some(tools), true)
        };

        log::debug!("LLM 流式请求: url={url}, model={}", self.config.model);

        // 发送请求
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Llm(format!("LLM 请求发送失败: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "<无法读取响应体>".to_string());
            return Err(Error::Llm(format!(
                "LLM API 返回错误 (HTTP {status_code}): {error_body}"
            )));
        }

        // 流式读取响应
        let mut stream = response.bytes_stream();
        let mut parser = SseParser::new();
        let mut content = String::new();
        let mut partial_tool_calls = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| Error::Llm(format!("读取流式响应失败: {e}")))?;

            parser.feed(&chunk);

            for event in parser.parse_events() {
                if SseParser::is_done(&event) {
                    // 流式结束
                    let final_content = if content.is_empty() {
                        None
                    } else {
                        Some(content.clone())
                    };

                    let tool_calls = if partial_tool_calls.is_empty() {
                        None
                    } else {
                        Some(partial_tool_calls.into_iter().map(convert_partial_to_tool_call).collect())
                    };

                    let response_msg = ChatMessage::assistant_with_tools(final_content, tool_calls.unwrap_or_default());
                    callback.on_complete(&content);
                    return Ok(response_msg);
                }

                // 提取内容增量
                if let Some(delta) = SseParser::extract_delta(&event) {
                    content.push_str(&delta);
                    callback.on_token(&delta);
                }

                // 提取工具调用增量
                if let Some(tool_delta) = SseParser::extract_tool_calls_delta(&event) {
                    accumulate_tool_calls(&mut partial_tool_calls, tool_delta);
                }
            }
        }

        // 流结束但没有 [DONE] 信号——仍然返回已收集的内容
        let final_content = if content.is_empty() {
            None
        } else {
            Some(content.clone())
        };

        let tool_calls = if partial_tool_calls.is_empty() {
            None
        } else {
            Some(partial_tool_calls.into_iter().map(convert_partial_to_tool_call).collect())
        };

        let response_msg = ChatMessage::assistant_with_tools(final_content, tool_calls.unwrap_or_default());
        callback.on_complete(&content);
        Ok(response_msg)
    }

    /// 非流式聊天
    ///
    /// 发送消息到 LLM，等待完整响应返回。
    ///
    /// # 参数
    /// - `messages`: 对话消息列表
    ///
    /// # 返回
    /// LLM 的完整响应消息
    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatMessage> {
        let url = self.provider.base_url(&self.config.endpoint);
        let headers = self.build_headers();

        let body = if self.provider == Provider::Anthropic {
            self.build_anthropic_body(messages, None, false)
        } else {
            self.build_body(messages, None, false)
        };

        log::debug!("LLM 非流式请求: url={url}, model={}", self.config.model);

        let mut last_error: Option<String> = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let status_code = status.as_u16();
                        let error_body = resp.text().await.unwrap_or_else(|_| "<无法读取响应体>".to_string());
                        let err = Error::Llm(format!("LLM API 返回错误 (HTTP {status_code}): {error_body}"));
                        last_error = Some(err.to_string());

                        if is_non_retryable(&err) {
                            return Err(err);
                        }
                        continue;
                    }

                    let resp_body: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| Error::Llm(format!("解析 LLM 响应 JSON 失败: {e}")))?;

                    return parse_non_streaming_response(&resp_body);
                }
                Err(e) => {
                    let err_str = format!("LLM 请求发送失败: {e}");
                    log::warn!("LLM 非流式请求第 {}/{} 次失败: {err_str}", attempt + 1, MAX_RETRIES);
                    last_error = Some(err_str);

                    if attempt + 1 < MAX_RETRIES {
                        let delay = Duration::from_millis(500 * 2u64.pow(attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(Error::Llm(format!(
            "LLM 非流式请求重试 {MAX_RETRIES} 次后仍失败: {}",
            last_error.unwrap_or_else(|| "未知错误".to_string())
        )))
    }

    /// 上下文压缩
    ///
    /// 使用 LLM 将系统提示词和长文本压缩为更短的摘要。
    /// 当上下文超过压缩阈值时自动调用。
    ///
    /// # 参数
    /// - `system_prompt`: 系统提示词
    /// - `text`: 需要压缩的文本
    ///
    /// # 返回
    /// 压缩后的摘要文本
    pub async fn compress(&self, system_prompt: &str, text: &str) -> Result<String> {
        let compression_prompt = format!(
            "请将以下内容压缩为简洁的摘要，保留所有关键信息，去除冗余内容。\n\n\
             --- 系统提示词 ---\n{system_prompt}\n\n\
             --- 待压缩内容 ---\n{text}"
        );

        let messages = vec![
            ChatMessage::system("你是一个文本压缩助手。你需要将长文本压缩为简短的摘要，保留所有关键信息。"),
            ChatMessage::user(&compression_prompt),
        ];

        let response = self.chat(&messages).await?;

        Ok(response.content.unwrap_or_default())
    }

    /// 获取当前提供商
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// 获取当前模型名称
    pub fn model(&self) -> &str {
        &self.config.model
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 将部分工具调用转换为完整的 ToolCall
///
/// # 参数
/// - `partial`: 已累积完成的 PartialToolCall
///
/// # 返回
/// 转换后的 ToolCall
fn convert_partial_to_tool_call(partial: super::sse::PartialToolCall) -> ToolCall {
    ToolCall {
        id: partial.id.unwrap_or_default(),
        function: FunctionCall {
            name: partial.name.unwrap_or_default(),
            arguments: partial.arguments.unwrap_or_default(),
        },
    }
}

/// 解析非流式响应
///
/// 从 OpenAI 兼容格式的响应 JSON 中提取消息内容。
fn parse_non_streaming_response(body: &serde_json::Value) -> Result<ChatMessage> {
    let choice = body
        .get("choices")
        .and_then(|c| c.get(0))
        .ok_or_else(|| Error::Llm("LLM 响应中缺少 choices 字段".to_string()))?;

    let message = choice
        .get("message")
        .ok_or_else(|| Error::Llm("LLM 响应中缺少 message 字段".to_string()))?;

    let content = message
        .get("content")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    // 提取工具调用
    let tool_calls = message
        .get("tool_calls")
        .and_then(|tc| tc.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let id = tc.get("id")?.as_str()?.to_string();
                    let func = tc.get("function")?;
                    let name = func.get("name")?.as_str()?.to_string();
                    let arguments = func
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .unwrap_or("{}")
                        .to_string();
                    Some(ToolCall {
                        id,
                        function: FunctionCall { name, arguments },
                    })
                })
                .collect::<Vec<_>>()
        });

    match tool_calls {
        Some(calls) if !calls.is_empty() => {
            Ok(ChatMessage::assistant_with_tools(content, calls))
        }
        _ => {
            Ok(ChatMessage::assistant(&content.unwrap_or_default()))
        }
    }
}

/// 判断错误是否不可重试
///
/// HTTP 4xx 错误（除 429 限流外）通常不可重试。
fn is_non_retryable(error: &Error) -> bool {
    let msg = match error {
        Error::Llm(msg) => msg.as_str(),
        _ => return false,
    };

    // HTTP 4xx 错误（401、403、404 等）不可重试
    // 但 429 (Too Many Requests) 可以重试
    if msg.contains("HTTP 401") || msg.contains("HTTP 403") || msg.contains("HTTP 404") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_from_kernel_config() {
        let kernel_config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "gpt-4", "api_base": "https://api.openai.com/v1", "api_key": "sk-test", "max_tokens": 8192}"#,
        )
        .unwrap();

        let llm_config = LlmConfig::from(&kernel_config);
        assert_eq!(llm_config.model, "gpt-4");
        assert_eq!(llm_config.endpoint, "https://api.openai.com/v1");
        assert_eq!(llm_config.api_key, "sk-test");
        assert_eq!(llm_config.max_tokens, 8192);
    }

    #[test]
    fn test_tool_definition_function() {
        let tool = ToolDefinition::function(
            "get_weather",
            "获取天气信息",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string", "description": "城市名称"}
                },
                "required": ["city"]
            }),
        );

        assert_eq!(tool.type_field, "function");
        assert_eq!(tool.function.name, "get_weather");
        assert_eq!(tool.function.description, "获取天气信息");
    }

    #[test]
    fn test_build_body() {
        let kernel_config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let client = LlmClient::new(&kernel_config).unwrap();
        let messages = vec![ChatMessage::user("Hello")];
        let body = client.build_body(&messages, None, true);

        assert_eq!(body["model"], "deepseek-chat");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 4096);
    }

    #[test]
    fn test_build_body_with_tools() {
        let kernel_config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "deepseek-chat", "api_key": "sk-test"}"#,
        )
        .unwrap();

        let client = LlmClient::new(&kernel_config).unwrap();
        let messages = vec![ChatMessage::user("What's the weather?")];
        let tools = vec![ToolDefinition::function(
            "get_weather",
            "Get weather",
            serde_json::json!({"type": "object", "properties": {}}),
        )];
        let body = client.build_body(&messages, Some(&tools), true);

        assert!(body["tools"].is_array());
        assert_eq!(body["tools"][0]["type"], "function");
    }

    #[test]
    fn test_parse_non_streaming_response() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }]
        });

        let msg = parse_non_streaming_response(&body).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content.as_deref(), Some("Hello! How can I help you?"));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_parse_non_streaming_response_with_tool_calls() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\": \"Beijing\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let msg = parse_non_streaming_response(&body).unwrap();
        assert_eq!(msg.role, "assistant");
        let calls = msg.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_abc");
        assert_eq!(calls[0].function.name, "get_weather");
        assert_eq!(calls[0].function.arguments, "{\"city\": \"Beijing\"}");
    }

    #[test]
    fn test_is_non_retryable() {
        assert!(is_non_retryable(&Error::Llm("HTTP 401: Unauthorized".to_string())));
        assert!(is_non_retryable(&Error::Llm("HTTP 403: Forbidden".to_string())));
        assert!(is_non_retryable(&Error::Llm("HTTP 404: Not Found".to_string())));
        assert!(!is_non_retryable(&Error::Llm("HTTP 429: Too Many Requests".to_string())));
        assert!(!is_non_retryable(&Error::Llm("connection refused".to_string())));
    }

    #[test]
    fn test_build_anthropic_body() {
        let kernel_config = KernelConfig::from_json(
            r#"{"data_dir": "/tmp", "model": "claude-3-sonnet", "api_base": "https://api.anthropic.com/v1", "api_key": "sk-ant-test"}"#,
        )
        .unwrap();

        let client = LlmClient::new(&kernel_config).unwrap();
        assert_eq!(client.provider(), Provider::Anthropic);

        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("Hello"),
        ];
        let body = client.build_anthropic_body(&messages, None, true);

        assert_eq!(body["system"], "You are helpful.");
        assert_eq!(body["stream"], true);
        // System message should be extracted from messages array
        let filtered = body["messages"].as_array().unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["role"], "user");
    }

    #[test]
    fn test_convert_partial_to_tool_call() {
        let partial = super::super::sse::PartialToolCall {
            index: 0,
            id: Some("call_123".to_string()),
            name: Some("search".to_string()),
            arguments: Some("{\"q\":\"test\"}".to_string()),
        };

        let tc = convert_partial_to_tool_call(partial);
        assert_eq!(tc.id, "call_123");
        assert_eq!(tc.function.name, "search");
        assert_eq!(tc.function.arguments, "{\"q\":\"test\"}");
    }
}
