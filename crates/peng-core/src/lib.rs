//! peng-core: 鹏智能体核心库
//!
//! 本库提供智能体的核心功能，包括：
//! - LLM 客户端（支持 SSE 流式输出）
//! - 工具路由（6 大领域：代码、媒体、文件、手机、网页、记忆）
//! - 记忆引擎（SQLite 后端）
//! - 技能匹配与注入
//! - 上下文压缩
//! - Agent 主循环
//!
//! # 核心类型
//!
//! - [`Error`] / [`Result`] — 统一错误处理
//! - [`ChatMessage`] / [`ToolCall`] / [`ToolResult`] — 对话与工具调用
//! - [`ToolDomain`] — 工具领域分类
//! - [`StreamCallback`] — 流式输出回调
//! - [`KernelStatus`] — 内核运行状态
//! - [`KernelConfig`] — 内核配置

pub mod types;
pub mod config;

// 子模块占位（后续逐步实现）
pub mod agent;
pub mod compression;
pub mod llm;
pub mod memory;
pub mod skills;
pub mod tools;

// 重导出关键类型
pub use types::{
    ChatMessage, Error, FunctionCall, KernelStatus, Result, StreamCallback, ToolCall, ToolDomain,
    ToolResult,
};
pub use config::KernelConfig;
pub use agent::AgentLoop;
