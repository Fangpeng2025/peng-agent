//! 回忆记忆 — 对话历史与情景记忆搜索
//!
//! RecallMemory 提供对话历史的持久化存储和搜索能力，
//! 支持按会话查询、全文搜索、最近消息获取等功能。

use crate::types::{ChatMessage, Error, Result};

// ============================================================================
// 回忆记忆条目
// ============================================================================

/// 回忆记忆条目
///
/// 存储单条对话消息，包含会话 ID、角色、内容和时间戳。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecallEntry {
    /// 消息 ID（自增）
    pub id: i64,
    /// 会话 ID
    pub session_id: String,
    /// 消息角色（system/user/assistant/tool）
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 时间戳（ISO 8601 格式字符串）
    pub timestamp: String,
    /// 嵌入摘要（可选，用于搜索优化）
    pub embedding_summary: Option<String>,
}

// ============================================================================
// 回忆记忆
// ============================================================================

/// 回忆记忆 — 对话历史与情景记忆
///
/// 提供对话消息的持久化存储，支持按会话查询、全文搜索和格式转换。
/// 数据通过 SQLite 持久化。
pub struct RecallMemory {
    /// SQLite 数据库路径
    db_path: String,
}

impl RecallMemory {
    /// 创建或打开回忆记忆
    ///
    /// 如果指定路径的 SQLite 数据库已存在，则打开已有数据库；
    /// 否则创建新的数据库和表结构。
    ///
    /// # 参数
    /// - `db_path`: SQLite 数据库文件路径
    ///
    /// # 返回
    /// 初始化成功返回 RecallMemory，失败返回 Error
    pub fn new(db_path: &str) -> Result<Self> {
        let memory = Self {
            db_path: db_path.to_string(),
        };
        memory.init_db()?;
        log::info!("回忆记忆初始化完成: path={db_path}");
        Ok(memory)
    }

    /// 初始化数据库表结构
    fn init_db(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS recall_memory (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                embedding_summary TEXT
            )",
            [],
        )
        .map_err(|e| Error::Sqlite(format!("创建 recall_memory 表失败: {e}")))?;

        // 创建索引加速查询
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_recall_session ON recall_memory(session_id)",
            [],
        )
        .map_err(|e| Error::Sqlite(format!("创建 recall_memory 会话索引失败: {e}")))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_recall_timestamp ON recall_memory(timestamp)",
            [],
        )
        .map_err(|e| Error::Sqlite(format!("创建 recall_memory 时间索引失败: {e}")))?;

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

    /// 存储一条消息
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `role`: 消息角色（system/user/assistant/tool）
    /// - `content`: 消息内容
    ///
    /// # 返回
    /// 插入成功返回消息 ID，失败返回 Error
    pub fn store_message(&self, session_id: &str, role: &str, content: &str) -> Result<i64> {
        let timestamp = Self::now_timestamp();

        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO recall_memory (session_id, role, content, timestamp, embedding_summary) VALUES (?1, ?2, ?3, ?4, NULL)",
            rusqlite::params![session_id, role, content, timestamp],
        )
        .map_err(|e| Error::Sqlite(format!("存储回忆消息失败: {e}")))?;

        let id = conn.last_insert_rowid();
        log::debug!("回忆消息存储: session={session_id}, role={role}, id={id}");
        Ok(id)
    }

    /// 获取会话的最近 N 条消息
    ///
    /// 按时间戳降序排列，返回最近的 `limit` 条消息。
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `limit`: 最大返回条数
    ///
    /// # 返回
    /// 最近的消息列表（按时间从旧到新排列）
    pub fn get_recent(&self, session_id: &str, limit: usize) -> Result<Vec<RecallEntry>> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, role, content, timestamp, embedding_summary \
                 FROM recall_memory \
                 WHERE session_id = ?1 \
                 ORDER BY id DESC \
                 LIMIT ?2",
            )
            .map_err(|e| Error::Sqlite(format!("准备查询最近消息失败: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![session_id, limit as i64], |row| {
                Ok(RecallEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    embedding_summary: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("查询最近消息失败: {e}")))?;

        let mut entries: Vec<RecallEntry> = rows
            .filter_map(|r| r.ok())
            .collect();

        // 反转为时间从旧到新
        entries.reverse();
        Ok(entries)
    }

    /// 搜索消息
    ///
    /// 使用 LIKE 模糊匹配消息内容，返回匹配的消息列表。
    ///
    /// # 参数
    /// - `query`: 搜索关键词
    /// - `limit`: 最大返回条数
    ///
    /// # 返回
    /// 匹配的消息列表
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<RecallEntry>> {
        let pattern = format!("%{query}%");
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, role, content, timestamp, embedding_summary \
                 FROM recall_memory \
                 WHERE content LIKE ?1 \
                 ORDER BY id DESC \
                 LIMIT ?2",
            )
            .map_err(|e| Error::Sqlite(format!("准备搜索消息失败: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok(RecallEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    embedding_summary: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("搜索消息失败: {e}")))?;

        let entries: Vec<RecallEntry> = rows
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// 获取会话的完整历史
    ///
    /// 返回指定会话的所有消息，按时间从旧到新排列。
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    ///
    /// # 返回
    /// 会话的所有消息列表
    pub fn get_session_history(&self, session_id: &str) -> Result<Vec<RecallEntry>> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, role, content, timestamp, embedding_summary \
                 FROM recall_memory \
                 WHERE session_id = ?1 \
                 ORDER BY id ASC",
            )
            .map_err(|e| Error::Sqlite(format!("准备查询会话历史失败: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok(RecallEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    embedding_summary: row.get(5)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("查询会话历史失败: {e}")))?;

        let entries: Vec<RecallEntry> = rows
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// 删除会话的所有消息
    ///
    /// # 参数
    /// - `session_id`: 要删除的会话 ID
    ///
    /// # 返回
    /// 操作成功返回 Ok(())，失败返回 Error
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "DELETE FROM recall_memory WHERE session_id = ?1",
            rusqlite::params![session_id],
        )
        .map_err(|e| Error::Sqlite(format!("删除会话消息失败: {e}")))?;

        log::info!("回忆记忆会话已删除: session={session_id}");
        Ok(())
    }

    /// 将会话消息转换为 ChatMessage 格式
    ///
    /// 获取最近 N 条消息并转换为 LLM API 使用的 ChatMessage 格式。
    /// 注意：仅转换有文本内容的消息，tool_call 等信息不在此处还原。
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `limit`: 最大返回条数
    ///
    /// # 返回
    /// ChatMessage 格式的消息列表
    pub fn to_chat_messages(&self, session_id: &str, limit: usize) -> Result<Vec<ChatMessage>> {
        let entries = self.get_recent(session_id, limit)?;
        let messages: Vec<ChatMessage> = entries
            .into_iter()
            .map(|e| {
                let msg = match e.role.as_str() {
                    "system" => ChatMessage::system(&e.content),
                    "user" => ChatMessage::user(&e.content),
                    "assistant" => ChatMessage::assistant(&e.content),
                    "tool" => ChatMessage::tool_result("unknown", &e.content),
                    _ => ChatMessage::user(&e.content), // 未知角色默认作为用户消息
                };
                msg
            })
            .collect();
        Ok(messages)
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
    fn test_recall_memory_store_and_get() {
        let dir = std::env::temp_dir().join("peng_test_recall_memory");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("recall_test.db");
        std::fs::remove_file(&db_path).ok();

        let mem = RecallMemory::new(db_path.to_str().unwrap()).unwrap();

        let id1 = mem.store_message("sess1", "user", "你好").unwrap();
        let id2 = mem.store_message("sess1", "assistant", "你好！有什么可以帮你？").unwrap();
        assert!(id1 < id2);

        let recent = mem.get_recent("sess1", 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].role, "user");
        assert_eq!(recent[0].content, "你好");
    }

    #[test]
    fn test_recall_memory_search() {
        let dir = std::env::temp_dir().join("peng_test_recall_search");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("recall_search_test.db");
        std::fs::remove_file(&db_path).ok();

        let mem = RecallMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.store_message("s1", "user", "今天天气怎么样").unwrap();
        mem.store_message("s1", "assistant", "北京今天晴天").unwrap();
        mem.store_message("s1", "user", "帮我写个程序").unwrap();

        let results = mem.search("天气", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("天气"));
    }

    #[test]
    fn test_recall_memory_delete_session() {
        let dir = std::env::temp_dir().join("peng_test_recall_delete");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("recall_delete_test.db");
        std::fs::remove_file(&db_path).ok();

        let mem = RecallMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.store_message("s1", "user", "hello").unwrap();
        mem.store_message("s2", "user", "world").unwrap();

        mem.delete_session("s1").unwrap();

        let s1_history = mem.get_session_history("s1").unwrap();
        assert!(s1_history.is_empty());

        let s2_history = mem.get_session_history("s2").unwrap();
        assert_eq!(s2_history.len(), 1);
    }

    #[test]
    fn test_recall_memory_to_chat_messages() {
        let dir = std::env::temp_dir().join("peng_test_recall_chat");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("recall_chat_test.db");
        std::fs::remove_file(&db_path).ok();

        let mem = RecallMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.store_message("s1", "system", "你是一个助手").unwrap();
        mem.store_message("s1", "user", "你好").unwrap();
        mem.store_message("s1", "assistant", "你好！").unwrap();

        let messages = mem.to_chat_messages("s1", 10).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[2].role, "assistant");
    }

    #[test]
    fn test_recall_memory_get_recent_limit() {
        let dir = std::env::temp_dir().join("peng_test_recall_limit");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("recall_limit_test.db");
        std::fs::remove_file(&db_path).ok();

        let mem = RecallMemory::new(db_path.to_str().unwrap()).unwrap();
        for i in 0..5 {
            mem.store_message("s1", "user", &format!("msg {}", i)).unwrap();
        }

        let recent = mem.get_recent("s1", 2).unwrap();
        assert_eq!(recent.len(), 2);
        // 应该是最近的两条（按时间从旧到新排列）
        assert_eq!(recent[0].content, "msg 3");
        assert_eq!(recent[1].content, "msg 4");
    }
}
