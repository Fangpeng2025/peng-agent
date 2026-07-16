//! 智能体主循环模块
//!
//! 提供智能体的核心对话循环：
//! - [`AgentLoop`] — 智能体主循环，管理完整的对话流程
//!
//! 工具路由由 [`crate::tools::ToolRouter`] 提供，不在此模块中重复定义。

pub mod agent_loop;

// 重导出关键类型
pub use agent_loop::AgentLoop;
