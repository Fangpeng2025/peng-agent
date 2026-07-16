//! 统一记忆引擎
//!
//! 整合核心记忆、回忆记忆和知识库，提供统一的记忆管理接口。
//! 支持为系统提示词生成上下文注入文本。

use crate::types::{Error, Result};

use super::core_memory::CoreMemory;
use super::knowledge::KnowledgeBase;
use super::recall_memory::RecallMemory;

// ============================================================================
// 统一记忆引擎
// ============================================================================

/// 统一记忆引擎
///
/// 整合三层记忆架构，提供统一的初始化、查询和上下文生成接口。
/// 所有数据文件存储在 `data_dir/memory/` 目录下。
pub struct MemoryEngine {
    /// 核心记忆
    core: CoreMemory,
    /// 回忆记忆
    recall: RecallMemory,
    /// 知识库
    knowledge: KnowledgeBase,
}

impl MemoryEngine {
    /// 创建或加载记忆引擎
    ///
    /// 在 `data_dir/memory/` 目录下创建/打开三个 SQLite 数据库：
    /// - `core.db` — 核心记忆
    /// - `recall.db` — 回忆记忆
    /// - `knowledge.db` — 知识库
    ///
    /// # 参数
    /// - `data_dir`: 数据根目录
    ///
    /// # 返回
    /// 初始化成功返回 MemoryEngine，失败返回 Error
    pub fn new(data_dir: &str) -> Result<Self> {
        let memory_dir = format!("{data_dir}/memory");
        std::fs::create_dir_all(&memory_dir)
            .map_err(|e| Error::Io(format!("创建记忆目录失败: {e}")))?;

        let core = CoreMemory::new(&format!("{memory_dir}/core.db"))?;
        let recall = RecallMemory::new(&format!("{memory_dir}/recall.db"))?;
        let knowledge = KnowledgeBase::new(&format!("{memory_dir}/knowledge.db"))?;

        log::info!("记忆引擎初始化完成: data_dir={data_dir}");

        Ok(Self {
            core,
            recall,
            knowledge,
        })
    }

    // ========================================================================
    // 核心记忆代理方法
    // ========================================================================

    /// 获取核心记忆引用
    pub fn core(&self) -> &CoreMemory {
        &self.core
    }

    /// 获取核心记忆可变引用
    pub fn core_mut(&mut self) -> &mut CoreMemory {
        &mut self.core
    }

    // ========================================================================
    // 回忆记忆代理方法
    // ========================================================================

    /// 获取回忆记忆引用
    pub fn recall(&self) -> &RecallMemory {
        &self.recall
    }

    // ========================================================================
    // 知识库代理方法
    // ========================================================================

    /// 获取知识库引用
    pub fn knowledge(&self) -> &KnowledgeBase {
        &self.knowledge
    }

    // ========================================================================
    // 统一接口
    // ========================================================================

    /// 为系统提示词生成记忆上下文
    ///
    /// 收集核心记忆和最近对话历史，格式化为可注入系统提示词的文本块。
    ///
    /// # 参数
    /// - `session_id`: 当前会话 ID
    /// - `recent_limit`: 获取最近消息的条数
    ///
    /// # 返回
    /// 格式化的记忆上下文文本
    pub fn get_context_for_prompt(&self, session_id: &str, recent_limit: usize) -> Result<String> {
        let mut context = String::new();

        // 1. 核心记忆
        let core_ctx = self.core.to_context_string();
        if !core_ctx.is_empty() {
            context.push_str(&core_ctx);
            context.push('\n');
        }

        // 2. 最近对话摘要
        let recent = self.recall.get_recent(session_id, recent_limit)?;
        if !recent.is_empty() {
            context.push_str("=== 最近对话 ===\n");
            for entry in &recent {
                let role_label = match entry.role.as_str() {
                    "user" => "用户",
                    "assistant" => "助手",
                    "system" => "系统",
                    "tool" => "工具",
                    _ => "未知",
                };
                // 截断过长的消息
                let content_preview = if entry.content.len() > 200 {
                    format!("{}...", &entry.content[..200])
                } else {
                    entry.content.clone()
                };
                context.push_str(&format!("[{role_label}] {content_preview}\n"));
            }
            context.push_str("=== 最近对话结束 ===\n");
        }

        Ok(context)
    }

    /// 存储用户消息到回忆记忆
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `role`: 消息角色
    /// - `content`: 消息内容
    pub fn store_message(&self, session_id: &str, role: &str, content: &str) -> Result<i64> {
        self.recall.store_message(session_id, role, content)
    }

    /// 保存核心记忆条目
    ///
    /// # 参数
    /// - `key`: 键名
    /// - `value`: 值
    /// - `category`: 分类
    pub fn save_core_memory(&mut self, key: &str, value: &str, category: &str) -> Result<()> {
        self.core.set(key, value, category)
    }

    /// 回忆核心记忆
    ///
    /// # 参数
    /// - `key`: 键名
    pub fn recall_core_memory(&self, key: &str) -> Option<String> {
        self.core.get(key).map(|e| e.value.clone())
    }

    /// 搜索知识库
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 最大返回条数
    pub fn search_knowledge(&self, query: &str, limit: usize) -> Result<Vec<crate::memory::KnowledgeEntry>> {
        self.knowledge.search(query, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_engine_new() {
        let dir = std::env::temp_dir().join("peng_test_memory_engine");
        std::fs::create_dir_all(&dir).ok();
        let data_dir = dir.to_str().unwrap();

        let engine = MemoryEngine::new(data_dir);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_memory_engine_core_operations() {
        let dir = std::env::temp_dir().join("peng_test_memory_engine_core");
        std::fs::create_dir_all(&dir).ok();
        let data_dir = dir.to_str().unwrap();

        let mut engine = MemoryEngine::new(data_dir).unwrap();
        engine.save_core_memory("name", "张三", "Personal").unwrap();

        let value = engine.recall_core_memory("name");
        assert_eq!(value, Some("张三".to_string()));
    }

    #[test]
    fn test_memory_engine_store_and_recall() {
        let dir = std::env::temp_dir().join("peng_test_memory_engine_recall2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok();
        let data_dir = dir.to_str().unwrap();

        let engine = MemoryEngine::new(data_dir).unwrap();
        let id = engine.store_message("sess1", "user", "你好").unwrap();
        assert!(id > 0);

        let recent = engine.recall().get_recent("sess1", 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "你好");
    }

    #[test]
    fn test_memory_engine_context_for_prompt() {
        let dir = std::env::temp_dir().join("peng_test_memory_engine_ctx");
        std::fs::create_dir_all(&dir).ok();
        let data_dir = dir.to_str().unwrap();

        let mut engine = MemoryEngine::new(data_dir).unwrap();
        engine.save_core_memory("name", "张三", "Personal").unwrap();
        engine.store_message("sess1", "user", "你好").unwrap();

        let ctx = engine.get_context_for_prompt("sess1", 10).unwrap();
        assert!(ctx.contains("用户核心记忆"));
        assert!(ctx.contains("name: 张三"));
        assert!(ctx.contains("最近对话"));
    }
}
