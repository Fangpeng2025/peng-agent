//! LLM 客户端模块
//!
//! 提供 LLM API 的完整客户端实现，包括：
//! - 提供商检测与配置适配
//! - SSE 流式事件解析
//! - 流式/非流式聊天接口
//! - 工具调用支持
//! - 上下文压缩

pub mod client;
pub mod provider;
pub mod sse;

// 重导出关键类型
pub use client::{FunctionDefinition, LlmClient, LlmConfig, ToolDefinition};
pub use provider::{detect_provider, Provider};
pub use sse::{accumulate_tool_calls, PartialToolCall, SseEvent, SseParser};
