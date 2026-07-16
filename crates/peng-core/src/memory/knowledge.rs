//! 知识库 — 长期知识文档存储与检索
//!
//! KnowledgeBase 提供长期知识文档的持久化存储和检索能力，
//! 支持标题/内容搜索、标签过滤等功能。

use crate::types::{Error, Result};
use rusqlite::OptionalExtension;

// ============================================================================
// 知识条目
// ============================================================================

/// 知识条目
///
/// 存储一条知识文档，包含标题、内容、标签和来源信息。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeEntry {
    /// 条目 ID（自增）
    pub id: i64,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 标签列表（JSON 数组字符串）
    pub tags: String,
    /// 来源
    pub source: String,
    /// 创建时间（ISO 8601 格式字符串）
    pub created_at: String,
}

impl KnowledgeEntry {
    /// 解析标签为 Vec<String>
    ///
    /// # 返回
    /// 解析成功返回标签列表，失败返回空列表
    pub fn tags_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.tags).unwrap_or_default()
    }
}

// ============================================================================
// 知识库
// ============================================================================

/// 知识库 — 长期知识文档存储
///
/// 提供知识文档的持久化存储，支持全文搜索和标签过滤。
/// 数据通过 SQLite 持久化。
pub struct KnowledgeBase {
    /// SQLite 数据库路径
    db_path: String,
}

impl KnowledgeBase {
    /// 创建或打开知识库
    ///
    /// 如果指定路径的 SQLite 数据库已存在，则打开已有数据库；
    /// 否则创建新的数据库和表结构。
    ///
    /// # 参数
    /// - `db_path`: SQLite 数据库文件路径
    ///
    /// # 返回
    /// 初始化成功返回 KnowledgeBase，失败返回 Error
    pub fn new(db_path: &str) -> Result<Self> {
        let kb = Self {
            db_path: db_path.to_string(),
        };
        kb.init_db()?;
        log::info!("知识库初始化完成: path={db_path}");
        Ok(kb)
    }

    /// 初始化数据库表结构
    fn init_db(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                source TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| Error::Sqlite(format!("创建 knowledge 表失败: {e}")))?;

        Ok(())
    }

    /// 打开 SQLite 连接
    fn open_connection(&self) -> Result<rusqlite::Connection> {
        rusqlite::Connection::open(&self.db_path)
            .map_err(|e| Error::Sqlite(format!("打开 SQLite 数据库失败 ({}): {e}", self.db_path)))
    }

    /// 获取当前 ISO 8601 时间戳
    fn now_timestamp() -> String {
        let now = std::time::SystemTime::now();
        let duration = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;
        let (year, month, day) = days_to_ymd(days);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    }

    /// 添加知识条目
    ///
    /// # 参数
    /// - `title`: 标题
    /// - `content`: 内容
    /// - `tags`: 标签列表
    /// - `source`: 来源
    ///
    /// # 返回
    /// 插入成功返回条目 ID，失败返回 Error
    pub fn add(&self, title: &str, content: &str, tags: &[String], source: &str) -> Result<i64> {
        let tags_json = serde_json::to_string(tags)
            .map_err(|e| Error::Json(format!("序列化标签失败: {e}")))?;
        let timestamp = Self::now_timestamp();

        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO knowledge (title, content, tags, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![title, content, tags_json, source, timestamp],
        )
        .map_err(|e| Error::Sqlite(format!("添加知识条目失败: {e}")))?;

        let id = conn.last_insert_rowid();
        log::debug!("知识条目添加: title={title}, id={id}");
        Ok(id)
    }

    /// 获取指定 ID 的知识条目
    ///
    /// # 参数
    /// - `id`: 条目 ID
    ///
    /// # 返回
    /// 存在时返回 Some(KnowledgeEntry)，不存在返回 None
    pub fn get(&self, id: i64) -> Result<Option<KnowledgeEntry>> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, tags, source, created_at \
                 FROM knowledge WHERE id = ?1",
            )
            .map_err(|e| Error::Sqlite(format!("准备查询知识条目失败: {e}")))?;

        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok(KnowledgeEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| Error::Sqlite(format!("查询知识条目失败: {e}")))?;

        Ok(result)
    }

    /// 搜索知识条目
    ///
    /// 在标题和内容中进行 LIKE 模糊匹配。
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 最大返回条数
    ///
    /// # 返回
    /// 匹配的知识条目列表
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        let pattern = format!("%{query}%");
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, tags, source, created_at \
                 FROM knowledge \
                 WHERE title LIKE ?1 OR content LIKE ?1 \
                 ORDER BY id DESC \
                 LIMIT ?2",
            )
            .map_err(|e| Error::Sqlite(format!("准备搜索知识条目失败: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok(KnowledgeEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("搜索知识条目失败: {e}")))?;

        let entries: Vec<KnowledgeEntry> = rows
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// 按标签搜索知识条目
    ///
    /// 使用 LIKE 模糊匹配 JSON 标签数组中的标签。
    ///
    /// # 参数
    /// - `tag`: 标签关键词
    ///
    /// # 返回
    /// 匹配的知识条目列表
    pub fn search_by_tag(&self, tag: &str) -> Result<Vec<KnowledgeEntry>> {
        let pattern = format!("%{tag}%");
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, tags, source, created_at \
                 FROM knowledge \
                 WHERE tags LIKE ?1 \
                 ORDER BY id DESC",
            )
            .map_err(|e| Error::Sqlite(format!("准备按标签搜索知识条目失败: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| {
                Ok(KnowledgeEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("按标签搜索知识条目失败: {e}")))?;

        let entries: Vec<KnowledgeEntry> = rows
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// 删除知识条目
    ///
    /// # 参数
    /// - `id`: 要删除的条目 ID
    ///
    /// # 返回
    /// 操作成功返回 Ok(())，失败返回 Error
    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM knowledge WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| Error::Sqlite(format!("删除知识条目失败: {e}")))?;

        log::debug!("知识条目已删除: id={id}");
        Ok(())
    }

    /// 列出所有知识条目
    ///
    /// # 返回
    /// 所有知识条目列表（按 ID 降序排列）
    pub fn list_all(&self) -> Result<Vec<KnowledgeEntry>> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, tags, source, created_at \
                 FROM knowledge \
                 ORDER BY id DESC",
            )
            .map_err(|e| Error::Sqlite(format!("准备列出知识条目失败: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(KnowledgeEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("列出知识条目失败: {e}")))?;

        let entries: Vec<KnowledgeEntry> = rows
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }
}

/// 将自 1970-01-01 以来的天数转换为年月日
fn days_to_ymd(total_days: u64) -> (u64, u64, u64) {
    let mut days = total_days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let mut month = 1u64;
    loop {
        let dim = days_in_month(month, year);
        if days < dim {
            return (year as u64, month, (days + 1) as u64);
        }
        days -= dim;
        month += 1;
    }
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(m: u64, y: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_add_and_get() {
        let dir = std::env::temp_dir().join("peng_test_knowledge");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("kb_test.db");
        std::fs::remove_file(&db_path).ok();

        let kb = KnowledgeBase::new(db_path.to_str().unwrap()).unwrap();
        let tags = vec!["Rust".to_string(), "编程".to_string()];
        let id = kb.add("Rust入门", "Rust 是一门系统编程语言...", &tags, "web").unwrap();
        assert!(id > 0);

        let entry = kb.get(id).unwrap().unwrap();
        assert_eq!(entry.title, "Rust入门");
        assert_eq!(entry.source, "web");
        assert_eq!(entry.tags_vec(), tags);
    }

    #[test]
    fn test_knowledge_base_search() {
        let dir = std::env::temp_dir().join("peng_test_knowledge_search");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("kb_search_test.db");
        std::fs::remove_file(&db_path).ok();

        let kb = KnowledgeBase::new(db_path.to_str().unwrap()).unwrap();
        kb.add("Rust入门", "Rust 编程语言入门教程", &["Rust".to_string()], "book").unwrap();
        kb.add("Python入门", "Python 编程语言入门教程", &["Python".to_string()], "book").unwrap();

        let results = kb.search("Rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust入门");
    }

    #[test]
    fn test_knowledge_base_search_by_tag() {
        let dir = std::env::temp_dir().join("peng_test_knowledge_tag");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("kb_tag_test.db");
        std::fs::remove_file(&db_path).ok();

        let kb = KnowledgeBase::new(db_path.to_str().unwrap()).unwrap();
        kb.add("文章1", "内容1", &["编程".to_string(), "Rust".to_string()], "web").unwrap();
        kb.add("文章2", "内容2", &["编程".to_string(), "Python".to_string()], "web").unwrap();

        let results = kb.search_by_tag("Rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "文章1");
    }

    #[test]
    fn test_knowledge_base_delete() {
        let dir = std::env::temp_dir().join("peng_test_knowledge_delete");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("kb_delete_test.db");
        std::fs::remove_file(&db_path).ok();

        let kb = KnowledgeBase::new(db_path.to_str().unwrap()).unwrap();
        let id = kb.add("测试", "内容", &[], "test").unwrap();
        assert!(kb.get(id).unwrap().is_some());

        kb.delete(id).unwrap();
        assert!(kb.get(id).unwrap().is_none());
    }

    #[test]
    fn test_knowledge_base_list_all() {
        let dir = std::env::temp_dir().join("peng_test_knowledge_list");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("kb_list_test.db");
        std::fs::remove_file(&db_path).ok();

        let kb = KnowledgeBase::new(db_path.to_str().unwrap()).unwrap();
        kb.add("条目1", "内容1", &[], "test").unwrap();
        kb.add("条目2", "内容2", &[], "test").unwrap();
        kb.add("条目3", "内容3", &[], "test").unwrap();

        let all = kb.list_all().unwrap();
        assert_eq!(all.len(), 3);
    }
}
