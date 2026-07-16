//! SSE（Server-Sent Events）事件解析器
//!
//! 用于解析 LLM 流式响应中的 SSE 事件，提取内容增量和工具调用增量。
//! 支持 OpenAI 兼容格式的流式输出。

use serde::Deserialize;

// ============================================================================
// SSE 事件结构
// ============================================================================

/// SSE 事件
///
/// 表示一个完整的 Server-Sent Event，包含事件类型、数据和可选 ID。
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// 事件类型（如 "message"、空字符串表示默认）
    pub event_type: Option<String>,
    /// 事件数据（可能为多行，已用换行符连接）
    pub data: String,
    /// 事件 ID
    pub id: Option<String>,
}

// ============================================================================
// 流式工具调用增量
// ============================================================================

/// 部分工具调用（流式增量）
///
/// 在流式响应中，工具调用的各字段（id、name、arguments）会分多次到达，
/// 每次只包含增量部分。通过 `index` 字段关联同一工具调用的多次增量。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PartialToolCall {
    /// 工具调用在列表中的索引，用于关联增量
    #[serde(default)]
    pub index: usize,
    /// 工具调用的唯一标识符（首次增量时出现）
    #[serde(default)]
    pub id: Option<String>,
    /// 函数名称（首次增量时出现）
    #[serde(default)]
    pub name: Option<String>,
    /// 函数参数增量片段（多次增量拼接成完整 JSON）
    #[serde(default)]
    pub arguments: Option<String>,
}

// ============================================================================
// SSE 解析器
// ============================================================================

/// SSE 事件解析器
///
/// 从字节流中增量解析 SSE 事件。SSE 协议格式：
/// ```text
/// event: message
/// data: {"choices":[...]}
/// id: 123
///
/// ```
/// 事件之间以空行分隔。
pub struct SseParser {
    /// 内部缓冲区
    buffer: String,
}

impl SseParser {
    /// 创建新的 SSE 解析器
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(4096),
        }
    }

    /// 向缓冲区追加字节
    ///
    /// # 参数
    /// - `chunk`: 从流中读取的字节片段
    pub fn feed(&mut self, chunk: &[u8]) {
        if let Ok(text) = std::str::from_utf8(chunk) {
            self.buffer.push_str(text);
        }
        // 非 UTF-8 字节静默丢弃（LLM 响应应始终为 UTF-8）
    }

    /// 从缓冲区中提取所有完整的 SSE 事件
    ///
    /// 解析缓冲区中的 SSE 文本，提取以空行分隔的完整事件。
    /// 未完成的事件保留在缓冲区中等待更多数据。
    ///
    /// # 返回
    /// 已解析的完整 SSE 事件列表
    pub fn parse_events(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();

        // SSE 事件以双换行符（\n\n）分隔
        while let Some(pos) = self.buffer.find("\n\n") {
            let block = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            if block.trim().is_empty() {
                continue;
            }

            let mut event_type: Option<String> = None;
            let mut data_parts: Vec<String> = Vec::new();
            let mut id: Option<String> = None;

            for line in block.lines() {
                let line = line.trim_end_matches('\r');
                if line.starts_with("event:") {
                    event_type = Some(line[6..].trim().to_string());
                } else if line.starts_with("data:") {
                    data_parts.push(line[5..].trim().to_string());
                } else if line.starts_with("id:") {
                    id = Some(line[3..].trim().to_string());
                }
                // 忽略其他 SSE 字段（retry:、comment 等）
            }

            if !data_parts.is_empty() {
                events.push(SseEvent {
                    event_type,
                    data: data_parts.join("\n"),
                    id,
                });
            }
        }

        events
    }

    /// 从 SSE 事件中提取内容增量（content delta）
    ///
    /// 解析 OpenAI 格式的流式响应 JSON，提取 `choices[0].delta.content` 字段。
    ///
    /// # 参数
    /// - `event`: SSE 事件
    ///
    /// # 返回
    /// 内容增量文本，若无内容增量则返回 None
    pub fn extract_delta(event: &SseEvent) -> Option<String> {
        let json: serde_json::Value = serde_json::from_str(&event.data).ok()?;

        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("delta"))
            .and_then(|d| d.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    }

    /// 从 SSE 事件中提取工具调用增量
    ///
    /// 解析 OpenAI 格式的流式响应 JSON，提取 `choices[0].delta.tool_calls` 字段。
    ///
    /// # 参数
    /// - `event`: SSE 事件
    ///
    /// # 返回
    /// 工具调用增量列表，若无工具调用增量则返回 None
    pub fn extract_tool_calls_delta(event: &SseEvent) -> Option<Vec<PartialToolCall>> {
        let json: serde_json::Value = serde_json::from_str(&event.data).ok()?;

        let tool_calls = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("delta"))
            .and_then(|d| d.get("tool_calls"))?
            .as_array()?;

        let mut result = Vec::with_capacity(tool_calls.len());
        for tc in tool_calls {
            let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let id = tc
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // function 字段可能在 delta 中
            let func = tc.get("function");
            let name = func
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            let arguments = func
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());

            result.push(PartialToolCall {
                index,
                id,
                name,
                arguments,
            });
        }

        Some(result)
    }

    /// 检查 SSE 事件是否为流式结束信号
    ///
    /// OpenAI 格式在流结束时发送 `data: [DONE]`。
    ///
    /// # 参数
    /// - `event`: SSE 事件
    ///
    /// # 返回
    /// 如果是 [DONE] 信号返回 true
    pub fn is_done(event: &SseEvent) -> bool {
        event.data.trim() == "[DONE]"
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 工具调用增量累积
// ============================================================================

/// 将流式工具调用增量合并到累积列表中
///
/// 根据 `index` 字段将增量合并到对应位置的工具调用中：
/// - 如果 `index` 超出当前列表长度，则扩展列表
/// - `id`、`name` 非空时覆盖
/// - `arguments` 追加拼接
///
/// # 参数
/// - `partial`: 已累积的工具调用列表
/// - `delta`: 本次收到的工具调用增量
pub fn accumulate_tool_calls(partial: &mut Vec<PartialToolCall>, delta: Vec<PartialToolCall>) {
    for d in delta {
        // 确保列表足够长
        while partial.len() <= d.index {
            partial.push(PartialToolCall::default());
        }

        let entry = &mut partial[d.index];

        // id 和 name 仅在首次增量时出现，非空则覆盖
        if d.id.is_some() {
            entry.id = d.id;
        }
        if d.name.is_some() {
            entry.name = d.name;
        }

        // arguments 是增量片段，追加拼接
        if let Some(args) = d.arguments {
            match &mut entry.arguments {
                Some(existing) => existing.push_str(&args),
                None => entry.arguments = Some(args),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_parser_basic() {
        let mut parser = SseParser::new();
        parser.feed(b"data: hello\n\n");
        let events = parser.parse_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_sse_parser_multi_line_data() {
        let mut parser = SseParser::new();
        parser.feed(b"data: line1\ndata: line2\n\n");
        let events = parser.parse_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn test_sse_parser_event_type() {
        let mut parser = SseParser::new();
        parser.feed(b"event: message\ndata: test\n\n");
        let events = parser.parse_events();
        assert_eq!(events[0].event_type.as_deref(), Some("message"));
    }

    #[test]
    fn test_sse_parser_incremental() {
        let mut parser = SseParser::new();
        parser.feed(b"data: hel");
        assert!(parser.parse_events().is_empty());
        parser.feed(b"lo\n\n");
        let events = parser.parse_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_sse_parser_multiple_events() {
        let mut parser = SseParser::new();
        parser.feed(b"data: first\n\ndata: second\n\n");
        let events = parser.parse_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_extract_delta() {
        let event = SseEvent {
            event_type: None,
            data: r#"{"choices":[{"delta":{"content":"Hello"}}]}"#.to_string(),
            id: None,
        };
        assert_eq!(SseParser::extract_delta(&event), Some("Hello".to_string()));
    }

    #[test]
    fn test_extract_delta_no_content() {
        let event = SseEvent {
            event_type: None,
            data: r#"{"choices":[{"delta":{"role":"assistant"}}]}"#.to_string(),
            id: None,
        };
        assert_eq!(SseParser::extract_delta(&event), None);
    }

    #[test]
    fn test_extract_tool_calls_delta() {
        let event = SseEvent {
            event_type: None,
            data: r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","function":{"name":"get_weather","arguments":""}}]}}]}"#.to_string(),
            id: None,
        };
        let result = SseParser::extract_tool_calls_delta(&event).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[0].id.as_deref(), Some("call_123"));
        assert_eq!(result[0].name.as_deref(), Some("get_weather"));
    }

    #[test]
    fn test_is_done() {
        let event = SseEvent {
            event_type: None,
            data: "[DONE]".to_string(),
            id: None,
        };
        assert!(SseParser::is_done(&event));

        let event2 = SseEvent {
            event_type: None,
            data: r#"{"choices":[]}"#.to_string(),
            id: None,
        };
        assert!(!SseParser::is_done(&event2));
    }

    #[test]
    fn test_accumulate_tool_calls() {
        let mut partial = Vec::new();

        // First delta: id + name
        let delta1 = vec![PartialToolCall {
            index: 0,
            id: Some("call_1".to_string()),
            name: Some("search".to_string()),
            arguments: None,
        }];
        accumulate_tool_calls(&mut partial, delta1);
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].id.as_deref(), Some("call_1"));
        assert_eq!(partial[0].name.as_deref(), Some("search"));
        assert!(partial[0].arguments.is_none());

        // Second delta: arguments fragment
        let delta2 = vec![PartialToolCall {
            index: 0,
            id: None,
            name: None,
            arguments: Some("{\"qu".to_string()),
        }];
        accumulate_tool_calls(&mut partial, delta2);
        assert_eq!(partial[0].arguments.as_deref(), Some("{\"qu"));

        // Third delta: more arguments
        let delta3 = vec![PartialToolCall {
            index: 0,
            id: None,
            name: None,
            arguments: Some("ery\":1}".to_string()),
        }];
        accumulate_tool_calls(&mut partial, delta3);
        assert_eq!(partial[0].arguments.as_deref(), Some("{\"query\":1}"));
    }

    #[test]
    fn test_accumulate_multiple_tool_calls() {
        let mut partial = Vec::new();

        let delta = vec![
            PartialToolCall {
                index: 0,
                id: Some("call_a".to_string()),
                name: Some("tool_a".to_string()),
                arguments: Some("{}".to_string()),
            },
            PartialToolCall {
                index: 1,
                id: Some("call_b".to_string()),
                name: Some("tool_b".to_string()),
                arguments: Some("{}".to_string()),
            },
        ];
        accumulate_tool_calls(&mut partial, delta);
        assert_eq!(partial.len(), 2);
        assert_eq!(partial[0].name.as_deref(), Some("tool_a"));
        assert_eq!(partial[1].name.as_deref(), Some("tool_b"));
    }
}
