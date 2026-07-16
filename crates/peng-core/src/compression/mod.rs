//! 上下文压缩模块
//!
//! 当对话上下文超过阈值时，自动压缩历史消息以保持对话在 token 限制内。
//! 支持两种压缩策略：
//! - 滑动窗口：保留系统提示词和最近消息，丢弃中间旧消息
//! - LLM 压缩：使用 LLM 将旧消息压缩为摘要

pub mod context_compressor;

// 重导出关键类型
pub use context_compressor::ContextCompressor;
