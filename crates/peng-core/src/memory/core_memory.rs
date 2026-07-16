//! 核心记忆 — 始终加载的键值存储，保存用户的关键事实
//!
//! CoreMemory 提供一个持久的、始终可用的键值存储，用于保存关于用户的核心信息，
//! 例如个人偏好、技能、上下文等。数据通过 SQLite 持久化。

use std::collections::HashMap;

use crate::types::{Error, Result};

// ============================================================================
// 核心记忆条目
// ============================================================================

/// 核心记忆条目分类
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CoreCategory {
    /// 个人信息
    Personal,
    /// 偏好设置
    Preference,
    /// 上下文信息
    Context,
    /// 技能
    Skill,
}

impl CoreCategory {
    /// 从字符串解析分类
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "personal" => CoreCategory::Personal,
            "preference" => CoreCategory::Preference,
            "context" => CoreCategory::Context,
            "skill" => CoreCategory::Skill,
            _ => CoreCategory::Context, // 默认分类
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            CoreCategory::Personal => "Personal",
            CoreCategory::Preference => "Preference",
            CoreCategory::Context => "Context",
            CoreCategory::Skill => "Skill",
        }
    }
}

impl std::fmt::Display for CoreCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 核心记忆条目
///
/// 保存关于用户的单个事实或偏好，包含分类、更新时间和重要性权重。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreEntry {
    /// 键名
    pub key: String,
    /// 值
    pub value: String,
    /// 分类
    pub category: CoreCategory,
    /// 更新时间（ISO 8601 格式字符串）
    pub updated_at: String,
    /// 重要性权重（0.0 - 1.0）
    pub importance: f64,
}

// ============================================================================
// 核心记忆
// ============================================================================

/// 核心记忆 — 始终加载的键值存储
///
/// 保存关于用户的核心事实，在每次对话时都会注入到系统提示词中。
/// 数据通过 SQLite 持久化，支持增删改查和分类过滤。
pub struct CoreMemory {
    /// 内存中的条目缓存
    entries: HashMap<String, CoreEntry>,
    /// SQLite 数据库路径
    db_path: String,
}

impl CoreMemory {
    /// 创建或加载核心记忆
    ///
    /// 如果指定路径的 SQLite 数据库已存在，则加载已有数据；
    /// 否则创建新的数据库和表结构。
    ///
    /// # 参数
    /// - `db_path`: SQLite 数据库文件路径
    ///
    /// # 返回
    /// 初始化成功返回 CoreMemory，失败返回 Error
    pub fn new(db_path: &str) -> Result<Self> {
        let mut memory = Self {
            entries: HashMap::new(),
            db_path: db_path.to_string(),
        };
        memory.init_db()?;
        memory.load_from_db()?;
        log::info!("核心记忆初始化完成: path={db_path}, entries={}", memory.entries.len());
        Ok(memory)
    }

    /// 初始化数据库表结构
    fn init_db(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS core_memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                category TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                importance REAL NOT NULL DEFAULT 0.5
            )",
            [],
        )
        .map_err(|e| Error::Sqlite(format!("创建 core_memory 表失败: {e}")))?;
        Ok(())
    }

    /// 从数据库加载所有条目到内存
    fn load_from_db(&mut self) -> Result<()> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare("SELECT key, value, category, updated_at, importance FROM core_memory")
            .map_err(|e| Error::Sqlite(format!("准备查询 core_memory 失败: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(CoreEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    category: CoreCategory::from_str_loose(&row.get::<_, String>(2)?),
                    updated_at: row.get(3)?,
                    importance: row.get(4)?,
                })
            })
            .map_err(|e| Error::Sqlite(format!("查询 core_memory 失败: {e}")))?;

        for entry in rows {
            let entry = entry.map_err(|e| Error::Sqlite(format!("读取 core_memory 行失败: {e}")))?;
            self.entries.insert(entry.key.clone(), entry);
        }

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
        // 简单格式化为 ISO 8601 近似格式
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;
        // 从 1970-01-01 计算年月日
        let (year, month, day) = days_to_ymd(days);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    }

    /// 设置（插入或更新）条目
    ///
    /// 如果 key 已存在则更新，否则插入新条目。
    /// 默认重要性为 0.5，更新时间为当前时间。
    ///
    /// # 参数
    /// - `key`: 条目键名
    /// - `value`: 条目值
    /// - `category`: 分类名称（Personal/Preference/Context/Skill）
    ///
    /// # 返回
    /// 操作成功返回 Ok(())，失败返回 Error
    pub fn set(&mut self, key: &str, value: &str, category: &str) -> Result<()> {
        let cat = CoreCategory::from_str_loose(category);
        let timestamp = Self::now_timestamp();
        let importance = 0.5;

        let entry = CoreEntry {
            key: key.to_string(),
            value: value.to_string(),
            category: cat,
            updated_at: timestamp.clone(),
            importance,
        };

        // 持久化到 SQLite
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO core_memory (key, value, category, updated_at, importance) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![key, value, category, timestamp, importance],
        )
        .map_err(|e| Error::Sqlite(format!("写入 core_memory 失败: {e}")))?;

        // 更新内存缓存
        self.entries.insert(key.to_string(), entry);
        log::debug!("核心记忆设置: key={key}, category={category}");
        Ok(())
    }

    /// 获取条目
    ///
    /// # 参数
    /// - `key`: 条目键名
    ///
    /// # 返回
    /// 存在时返回条目引用，不存在返回 None
    pub fn get(&self, key: &str) -> Option<&CoreEntry> {
        self.entries.get(key)
    }

    /// 删除条目
    ///
    /// # 参数
    /// - `key`: 要删除的条目键名
    ///
    /// # 返回
    /// 操作成功返回 Ok(())，失败返回 Error
    pub fn remove(&mut self, key: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM core_memory WHERE key = ?1", rusqlite::params![key])
            .map_err(|e| Error::Sqlite(format!("删除 core_memory 条目失败: {e}")))?;

        self.entries.remove(key);
        log::debug!("核心记忆删除: key={key}");
        Ok(())
    }

    /// 按分类列出条目
    ///
    /// # 参数
    /// - `category`: 分类名称
    ///
    /// # 返回
    /// 匹配分类的条目引用列表
    pub fn list_by_category(&self, category: &str) -> Vec<&CoreEntry> {
        let cat = CoreCategory::from_str_loose(category);
        self.entries
            .values()
            .filter(|e| e.category == cat)
            .collect()
    }

    /// 列出所有条目
    ///
    /// # 返回
    /// 所有条目的引用列表
    pub fn list_all(&self) -> Vec<&CoreEntry> {
        self.entries.values().collect()
    }

    /// 将所有条目格式化为上下文文本块
    ///
    /// 用于注入到系统提示词中，让 LLM 了解用户的核心信息。
    /// 格式为分类分组的键值对列表。
    ///
    /// # 返回
    /// 格式化的上下文文本
    pub fn to_context_string(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut output = String::from("=== 用户核心记忆 ===\n");

        // 按分类分组
        let categories = [
            CoreCategory::Personal,
            CoreCategory::Preference,
            CoreCategory::Context,
            CoreCategory::Skill,
        ];

        for cat in &categories {
            let entries: Vec<&CoreEntry> = self
                .entries
                .values()
                .filter(|e| &e.category == cat)
                .collect();

            if entries.is_empty() {
                continue;
            }

            output.push_str(&format!("【{}】\n", cat));
            for entry in entries {
                output.push_str(&format!("- {}: {}\n", entry.key, entry.value));
            }
        }

        output.push_str("=== 核心记忆结束 ===\n");
        output
    }

    /// 持久化所有内存条目到 SQLite
    ///
    /// 将内存中的所有条目写入数据库。通常在批量修改后调用。
    ///
    /// # 返回
    /// 操作成功返回 Ok(())，失败返回 Error
    pub fn persist(&self) -> Result<()> {
        let mut conn = self.open_connection()?;

        // 使用事务批量写入
        let tx = conn
            .transaction()
            .map_err(|e| Error::Sqlite(format!("开始事务失败: {e}")))?;

        // 先清空再写入
        tx.execute("DELETE FROM core_memory", [])
            .map_err(|e| Error::Sqlite(format!("清空 core_memory 表失败: {e}")))?;

        for entry in self.entries.values() {
            tx.execute(
                "INSERT INTO core_memory (key, value, category, updated_at, importance) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    entry.key,
                    entry.value,
                    entry.category.as_str(),
                    entry.updated_at,
                    entry.importance
                ],
            )
            .map_err(|e| Error::Sqlite(format!("持久化 core_memory 条目失败: {e}")))?;
        }

        tx.commit()
            .map_err(|e| Error::Sqlite(format!("提交事务失败: {e}")))?;
        log::debug!("核心记忆持久化完成: {} 条", self.entries.len());
        Ok(())
    }

    /// 从 SQLite 数据库加载核心记忆
    ///
    /// 重新从数据库加载所有条目到内存，覆盖当前内存中的数据。
    ///
    /// # 参数
    /// - `db_path`: SQLite 数据库文件路径
    ///
    /// # 返回
    /// 加载成功返回 CoreMemory，失败返回 Error
    pub fn load(db_path: &str) -> Result<Self> {
        Self::new(db_path)
    }

    /// 获取条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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
    fn test_core_category_from_str() {
        assert_eq!(CoreCategory::from_str_loose("personal"), CoreCategory::Personal);
        assert_eq!(CoreCategory::from_str_loose("PREFERENCE"), CoreCategory::Preference);
        assert_eq!(CoreCategory::from_str_loose("Context"), CoreCategory::Context);
        assert_eq!(CoreCategory::from_str_loose("skill"), CoreCategory::Skill);
        assert_eq!(CoreCategory::from_str_loose("unknown"), CoreCategory::Context);
    }

    #[test]
    fn test_core_category_as_str() {
        assert_eq!(CoreCategory::Personal.as_str(), "Personal");
        assert_eq!(CoreCategory::Preference.as_str(), "Preference");
        assert_eq!(CoreCategory::Context.as_str(), "Context");
        assert_eq!(CoreCategory::Skill.as_str(), "Skill");
    }

    #[test]
    fn test_core_memory_new_and_set() {
        let dir = std::env::temp_dir().join("peng_test_core_memory");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_test.db");
        // 清理旧文件
        std::fs::remove_file(&db_path).ok();

        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        assert!(mem.is_empty());

        mem.set("name", "张三", "Personal").unwrap();
        mem.set("language", "中文", "Preference").unwrap();
        assert_eq!(mem.len(), 2);

        let entry = mem.get("name").unwrap();
        assert_eq!(entry.value, "张三");
        assert_eq!(entry.category, CoreCategory::Personal);
    }

    #[test]
    fn test_core_memory_remove() {
        let dir = std::env::temp_dir().join("peng_test_core_memory_rm");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_rm_test.db");
        std::fs::remove_file(&db_path).ok();

        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.set("key1", "val1", "Context").unwrap();
        assert_eq!(mem.len(), 1);

        mem.remove("key1").unwrap();
        assert!(mem.is_empty());
        assert!(mem.get("key1").is_none());
    }

    #[test]
    fn test_core_memory_list_by_category() {
        let dir = std::env::temp_dir().join("peng_test_core_memory_cat");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_cat_test.db");
        std::fs::remove_file(&db_path).ok();

        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.set("name", "张三", "Personal").unwrap();
        mem.set("age", "25", "Personal").unwrap();
        mem.set("lang", "中文", "Preference").unwrap();

        let personal = mem.list_by_category("Personal");
        assert_eq!(personal.len(), 2);

        let preference = mem.list_by_category("Preference");
        assert_eq!(preference.len(), 1);
    }

    #[test]
    fn test_core_memory_to_context_string() {
        let dir = std::env::temp_dir().join("peng_test_core_memory_ctx");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_ctx_test.db");
        std::fs::remove_file(&db_path).ok();

        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        assert!(mem.to_context_string().is_empty());

        mem.set("name", "张三", "Personal").unwrap();
        mem.set("lang", "中文", "Preference").unwrap();

        let ctx = mem.to_context_string();
        assert!(ctx.contains("用户核心记忆"));
        assert!(ctx.contains("name: 张三"));
        assert!(ctx.contains("lang: 中文"));
    }

    #[test]
    fn test_core_memory_persist_and_reload() {
        let dir = std::env::temp_dir().join("peng_test_core_memory_persist");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_persist_test.db");
        std::fs::remove_file(&db_path).ok();

        // 写入
        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.set("key1", "val1", "Context").unwrap();
        mem.set("key2", "val2", "Skill").unwrap();
        mem.persist().unwrap();

        // 重新加载
        let mem2 = CoreMemory::load(db_path.to_str().unwrap()).unwrap();
        assert_eq!(mem2.len(), 2);
        assert_eq!(mem2.get("key1").unwrap().value, "val1");
        assert_eq!(mem2.get("key2").unwrap().value, "val2");
    }

    #[test]
    fn test_core_memory_upsert() {
        let dir = std::env::temp_dir().join("peng_test_core_memory_upsert");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("core_upsert_test.db");
        std::fs::remove_file(&db_path).ok();

        let mut mem = CoreMemory::new(db_path.to_str().unwrap()).unwrap();
        mem.set("key1", "old_value", "Context").unwrap();
        assert_eq!(mem.get("key1").unwrap().value, "old_value");

        mem.set("key1", "new_value", "Personal").unwrap();
        assert_eq!(mem.get("key1").unwrap().value, "new_value");
        assert_eq!(mem.get("key1").unwrap().category, CoreCategory::Personal);
        assert_eq!(mem.len(), 1);
    }
}
