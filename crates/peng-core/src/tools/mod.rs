//! 工具路由模块
//!
//! 将智能体可调用的工具划分为六大领域，通过 [`ToolRouter`] 统一路由和分发：
//! - **代码领域** ([`code`]) — 代码执行：shell、Python、Node
//! - **媒体领域** ([`media`]) — 图片/音频/视频处理（ShellBridge/ffmpeg）
//! - **文件领域** ([`file`]) — 文件读写、目录操作（Rust 原生）
//! - **手机领域** ([`phone`]) — 设备控制、系统设置（Kotlin 回调）
//! - **网页领域** ([`web`]) — 网络请求、搜索（reqwest）
//! - **记忆领域** ([`memory`]) — 知识存储与检索

pub mod code;
pub mod file;
pub mod media;
pub mod memory;
pub mod phone;
pub mod router;
pub mod web;

// 重导出核心类型
pub use router::{ToolHandler, ToolRouter};
pub use crate::types::ToolDomain;
