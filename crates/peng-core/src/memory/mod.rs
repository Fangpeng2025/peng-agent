//! 记忆引擎模块
//!
//! 提供三层记忆架构：
//! - [`CoreMemory`] — 核心记忆：始终加载的键值存储，保存用户关键事实
//! - [`RecallMemory`] — 回忆记忆：对话历史与情景记忆搜索
//! - [`KnowledgeBase`] — 知识库：长期知识文档存储与检索
//! - [`MemoryEngine`] — 统一记忆引擎，整合三层记忆

pub mod core_memory;
pub mod recall_memory;
pub mod knowledge;
pub mod engine;

// 重导出关键类型
pub use core_memory::{CoreMemory, CoreEntry, CoreCategory};
pub use recall_memory::{RecallMemory, RecallEntry};
pub use knowledge::{KnowledgeBase, KnowledgeEntry};
pub use engine::MemoryEngine;
