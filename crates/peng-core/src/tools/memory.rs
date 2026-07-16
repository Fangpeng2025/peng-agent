//! 记忆领域工具处理器
//!
//! 提供知识存储与检索工具：
//! - `save_memory` — 保存记忆条目
//! - `recall_memory` — 按键名检索记忆
//! - `search_memory` — 搜索记忆（子串匹配）
//! - `delete_memory` — 删除记忆条目
//! - `list_memories` — 列出所有记忆键名
//!
//! 当前使用内存 HashMap 作为存储后端。
//! TODO: 后续将替换为 SQLite 后端的记忆模块（crate::memory）。

use std::collections::HashMap;
use std::sync::Mutex;

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// MemoryEntry
// ============================================================================

/// 记忆条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MemoryEntry {
    /// 记忆内容
    content: String,
    /// 标签列表
    tags: Vec<String>,
    /// 创建时间（Unix 时间戳）
    created_at: u64,
}

// ============================================================================
// MemoryHandler
// ============================================================================

/// 记忆领域工具处理器
///
/// 当前使用内存 HashMap 存储，进程退出后数据丢失。
/// 后续将替换为 SQLite 持久化存储。
pub struct MemoryHandler {
    /// 内存存储：key → MemoryEntry
    store: Mutex<HashMap<String, MemoryEntry>>,
}

impl MemoryHandler {
    /// 创建记忆领域处理器
    pub fn new(_config: &KernelConfig) -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    /// 保存记忆
    fn save_memory(&self, params: &str) -> Result<String> {
        let p: SaveMemoryParams = parse_params(params)?;
        let key = p.key;

        let entry = MemoryEntry {
            content: p.content,
            tags: p.tags.unwrap_or_default(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut store = self.store.lock().map_err(|e| {
            Error::Tool(format!("记忆存储锁获取失败: {e}"))
        })?;

        let is_update = store.contains_key(&key);
        store.insert(key.clone(), entry);

        if is_update {
            Ok(format!("记忆已更新: {key}"))
        } else {
            Ok(format!("记忆已保存: {key}"))
        }
    }

    /// 检索记忆
    fn recall_memory(&self, params: &str) -> Result<String> {
        let p: KeyParams = parse_params(params)?;

        let store = self.store.lock().map_err(|e| {
            Error::Tool(format!("记忆存储锁获取失败: {e}"))
        })?;

        match store.get(&p.key) {
            Some(entry) => {
                let result = serde_json::json!({
                    "key": p.key,
                    "content": entry.content,
                    "tags": entry.tags,
                    "created_at": entry.created_at,
                });
                Ok(serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|e| format!("{{\"error\": \"序列化失败: {e}\"}}")))
            }
            None => Ok(serde_json::json!({
                "status": "not_found",
                "key": p.key,
                "message": format!("未找到记忆: {}", p.key)
            })
            .to_string()),
        }
    }

    /// 搜索记忆
    ///
    /// 在所有记忆条目的内容和标签中进行子串匹配。
    fn search_memory(&self, params: &str) -> Result<String> {
        let p: SearchParams = parse_params(params)?;
        let query = p.query.to_lowercase();

        let store = self.store.lock().map_err(|e| {
            Error::Tool(format!("记忆存储锁获取失败: {e}"))
        })?;

        let results: Vec<serde_json::Value> = store
            .iter()
            .filter(|(_, entry)| {
                // 在内容中搜索
                let content_match = entry.content.to_lowercase().contains(&query);
                // 在标签中搜索
                let tag_match = entry.tags.iter().any(|t| t.to_lowercase().contains(&query));
                content_match || tag_match
            })
            .map(|(key, entry)| {
                serde_json::json!({
                    "key": key,
                    "content": entry.content,
                    "tags": entry.tags,
                    "created_at": entry.created_at,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "query": p.query,
            "count": results.len(),
            "results": results,
        }))
        .unwrap_or_else(|e| format!("{{\"error\": \"序列化失败: {e}\"}}")))
    }

    /// 删除记忆
    fn delete_memory(&self, params: &str) -> Result<String> {
        let p: KeyParams = parse_params(params)?;

        let mut store = self.store.lock().map_err(|e| {
            Error::Tool(format!("记忆存储锁获取失败: {e}"))
        })?;

        match store.remove(&p.key) {
            Some(_) => Ok(format!("记忆已删除: {}", p.key)),
            None => Ok(format!("记忆不存在: {}", p.key)),
        }
    }

    /// 列出所有记忆键名
    fn list_memories(&self, _params: &str) -> Result<String> {
        let store = self.store.lock().map_err(|e| {
            Error::Tool(format!("记忆存储锁获取失败: {e}"))
        })?;

        let keys: Vec<&String> = store.keys().collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "count": keys.len(),
            "keys": keys,
        }))
        .unwrap_or_else(|e| format!("{{\"error\": \"序列化失败: {e}\"}}")))
    }
}

impl ToolHandler for MemoryHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Memory
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "save_memory" => self.save_memory(params),
            "recall_memory" => self.recall_memory(params),
            "search_memory" => self.search_memory(params),
            "delete_memory" => self.delete_memory(params),
            "list_memories" => self.list_memories(params),
            _ => Err(Error::Tool(format!(
                "记忆领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "save_memory",
                "保存记忆条目。存储键值对形式的记忆，支持标签分类。如果键已存在则更新内容。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "记忆键名（唯一标识符）"
                        },
                        "content": {
                            "type": "string",
                            "description": "记忆内容"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "标签列表，用于分类和检索",
                            "default": []
                        }
                    },
                    "required": ["key", "content"]
                }),
            ),
            ToolDefinition::function(
                "recall_memory",
                "按键名检索记忆。返回指定键的记忆内容、标签和创建时间。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "要检索的记忆键名"
                        }
                    },
                    "required": ["key"]
                }),
            ),
            ToolDefinition::function(
                "search_memory",
                "搜索记忆。在所有记忆的内容和标签中进行子串匹配搜索。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词（子串匹配）"
                        }
                    },
                    "required": ["query"]
                }),
            ),
            ToolDefinition::function(
                "delete_memory",
                "删除指定键名的记忆条目。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "要删除的记忆键名"
                        }
                    },
                    "required": ["key"]
                }),
            ),
            ToolDefinition::function(
                "list_memories",
                "列出所有已保存的记忆键名。",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    }
}

// ============================================================================
// 参数结构
// ============================================================================

/// 保存记忆参数
#[derive(Debug, serde::Deserialize)]
struct SaveMemoryParams {
    key: String,
    content: String,
    tags: Option<Vec<String>>,
}

/// 键名参数
#[derive(Debug, serde::Deserialize)]
struct KeyParams {
    key: String,
}

/// 搜索参数
#[derive(Debug, serde::Deserialize)]
struct SearchParams {
    query: String,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 通用参数解析
fn parse_params<'a, T: serde::Deserialize<'a>>(params: &'a str) -> Result<T> {
    serde_json::from_str(params).map_err(|e| Error::Tool(format!("参数解析失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_memory_handler_domain() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::Memory);
    }

    #[test]
    fn test_memory_handler_tool_definitions() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 5);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"save_memory"));
        assert!(names.contains(&"recall_memory"));
        assert!(names.contains(&"search_memory"));
        assert!(names.contains(&"delete_memory"));
        assert!(names.contains(&"list_memories"));
    }

    #[test]
    fn test_save_and_recall_memory() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);

        // 保存
        let result = handler.execute(
            "save_memory",
            r#"{"key": "test_key", "content": "hello world", "tags": ["test"]}"#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("已保存"));

        // 检索
        let result = handler.execute("recall_memory", r#"{"key": "test_key"}"#);
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["content"], "hello world");
        assert_eq!(json["tags"][0], "test");
    }

    #[test]
    fn test_recall_nonexistent_memory() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);

        let result = handler.execute("recall_memory", r#"{"key": "nonexistent"}"#);
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["status"], "not_found");
    }

    #[test]
    fn test_search_memory() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);

        // 保存几条记忆
        let _ = handler.execute(
            "save_memory",
            r#"{"key": "rust_info", "content": "Rust is a systems programming language", "tags": ["programming"]}"#,
        );
        let _ = handler.execute(
            "save_memory",
            r#"{"key": "python_info", "content": "Python is a scripting language", "tags": ["programming"]}"#,
        );

        // 搜索
        let result = handler.execute("search_memory", r#"{"query": "rust"}"#);
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["count"], 1);

        // 按标签搜索
        let result = handler.execute("search_memory", r#"{"query": "programming"}"#);
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["count"], 2);
    }

    #[test]
    fn test_delete_memory() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);

        // 保存
        let _ = handler.execute(
            "save_memory",
            r#"{"key": "to_delete", "content": "will be deleted"}"#,
        );

        // 删除
        let result = handler.execute("delete_memory", r#"{"key": "to_delete"}"#);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("已删除"));

        // 确认已删除
        let result = handler.execute("recall_memory", r#"{"key": "to_delete"}"#);
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["status"], "not_found");
    }

    #[test]
    fn test_list_memories() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);

        // 保存几条记忆
        let _ = handler.execute(
            "save_memory",
            r#"{"key": "list_test_1", "content": "content 1"}"#,
        );
        let _ = handler.execute(
            "save_memory",
            r#"{"key": "list_test_2", "content": "content 2"}"#,
        );

        let result = handler.execute("list_memories", "{}");
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(json["count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_memory_handler_unknown_tool() {
        let config = test_config();
        let handler = MemoryHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }
}
