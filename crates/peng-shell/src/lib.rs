//! peng-shell: Termux ShellBridge
//!
//! 本库提供 Termux 环境下的命令执行桥接功能，包括：
//! - ShellBridge: 在 Termux 环境中执行 shell/python/node 命令
//! - BootstrapManager: Termux 环境引导与验证
//!
//! # 核心类型
//!
//! - [`ShellBridge`] — Termux 命令执行桥
//! - [`ShellResult`] — 命令执行结果
//! - [`BootstrapStatus`] — Termux 环境引导状态
//! - [`BootstrapManager`] — Termux 环境引导管理器

pub mod bridge;
pub mod bootstrap;

// 重导出关键类型
pub use bridge::{ShellBridge, ShellResult, BootstrapStatus};
pub use bootstrap::BootstrapManager;
