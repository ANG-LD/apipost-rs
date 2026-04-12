//! 数据库模块
//!
//! 负责SQLite数据库的初始化和数据访问
//! 存储历史记录、环境变量、收藏请求等数据

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// 数据库管理器
pub struct Database {
    /// 数据库连接
    conn: Mutex<Connection>,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new(path: &str) -> Result<Self> {
        let path = Self::expand_path(path);
        let conn = Connection::open(&path)
            .with_context(|| format!("无法打开数据库: {}", path.display()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_tables()?;
        Ok(db)
    }

    /// 展开路径中的~符号
    fn expand_path(path: &str) -> PathBuf {
        if let Some(stripped) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(stripped);
            }
        }
        PathBuf::from(path)
    }

    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒，无法获取连接"))?;

        // 创建历史记录表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                headers TEXT,
                body TEXT,
                response_status INTEGER,
                response_headers TEXT,
                response_body TEXT,
                response_time_ms INTEGER,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建环境变量表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS environments (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                variables TEXT NOT NULL,
                is_active INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建收藏请求表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS saved_requests (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                headers TEXT,
                body TEXT,
                description TEXT,
                folder_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建文件夹表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS folders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_saved_requests_folder ON saved_requests(folder_id)",
            [],
        )?;

        Ok(())
    }

    // ==================== 历史记录操作 ====================

    /// 添加历史记录
    pub fn add_history(&self, entry: &HistoryEntry) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO history (id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.method,
                entry.url,
                entry.headers,
                entry.body,
                entry.response_status,
                entry.response_headers,
                entry.response_body,
                entry.response_time_ms,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 获取历史记录列表
    pub fn get_history(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, created_at
             FROM history ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;

        let entries = stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                headers: row.get(3)?,
                body: row.get(4)?,
                response_status: row.get(5)?,
                response_headers: row.get(6)?,
                response_body: row.get(7)?,
                response_time_ms: row.get(8)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// 搜索历史记录
    pub fn search_history(&self, keyword: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, created_at
             FROM history
             WHERE url LIKE ?1 OR method LIKE ?1
             ORDER BY created_at DESC LIMIT ?2"
        )?;

        let pattern = format!("%{}%", keyword);
        let entries = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                headers: row.get(3)?,
                body: row.get(4)?,
                response_status: row.get(5)?,
                response_headers: row.get(6)?,
                response_body: row.get(7)?,
                response_time_ms: row.get(8)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// 删除历史记录
    pub fn delete_history(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 清空所有历史记录
    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    // ==================== 环境变量操作 ====================

    /// 添加或更新环境
    pub fn save_environment(&self, env: &Environment) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO environments (id, name, variables, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                variables = excluded.variables,
                is_active = excluded.is_active,
                updated_at = excluded.updated_at",
            params![
                env.id,
                env.name,
                env.variables,
                env.is_active as i32,
                env.created_at.to_rfc3339(),
                env.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 获取所有环境
    pub fn get_environments(&self) -> Result<Vec<Environment>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, variables, is_active, created_at, updated_at FROM environments ORDER BY name"
        )?;

        let envs = stmt.query_map([], |row| {
            Ok(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                variables: row.get(2)?,
                is_active: row.get::<_, i32>(3)? == 1,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(envs)
    }

    /// 获取当前激活的环境
    pub fn get_active_environment(&self) -> Result<Option<Environment>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, variables, is_active, created_at, updated_at FROM environments WHERE is_active = 1 LIMIT 1"
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                variables: row.get(2)?,
                is_active: true,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// 设置激活环境
    pub fn set_active_environment(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        // 先取消所有激活状态
        conn.execute("UPDATE environments SET is_active = 0", [])?;
        // 激活指定环境
        conn.execute("UPDATE environments SET is_active = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 删除环境
    pub fn delete_environment(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("DELETE FROM environments WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ==================== 收藏请求操作 ====================

    /// 保存请求到收藏
    pub fn save_request(&self, request: &SavedRequest) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO saved_requests (id, name, method, url, headers, body, description, folder_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                method = excluded.method,
                url = excluded.url,
                headers = excluded.headers,
                body = excluded.body,
                description = excluded.description,
                folder_id = excluded.folder_id,
                updated_at = excluded.updated_at",
            params![
                request.id,
                request.name,
                request.method,
                request.url,
                request.headers,
                request.body,
                request.description,
                request.folder_id,
                request.created_at.to_rfc3339(),
                request.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 获取所有收藏请求
    pub fn get_saved_requests(&self) -> Result<Vec<SavedRequest>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, method, url, headers, body, description, folder_id, created_at, updated_at
             FROM saved_requests ORDER BY updated_at DESC"
        )?;

        let requests = stmt.query_map([], |row| {
            Ok(SavedRequest {
                id: row.get(0)?,
                name: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                headers: row.get(4)?,
                body: row.get(5)?,
                description: row.get(6)?,
                folder_id: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(requests)
    }

    /// 删除收藏请求
    pub fn delete_saved_request(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("DELETE FROM saved_requests WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ==================== 数据结构定义 ====================

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub method: String,
    pub url: String,
    pub headers: Option<String>,
    pub body: Option<String>,
    pub response_status: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub response_time_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// 环境变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: String,  // JSON格式的键值对
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 保存的请求（收藏）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Option<String>,
    pub body: Option<String>,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct TestDb {
        db: Database,
        #[allow(dead_code)]
        temp_dir: tempfile::TempDir,
    }

    impl TestDb {
        fn new() -> Self {
            let temp_dir = tempdir().unwrap();
            let db = Database::new(&temp_dir.path().join("test.db").to_string_lossy()).unwrap();
            Self { db, temp_dir }
        }
    }

    #[test]
    fn test_history_operations() {
        let test_db = TestDb::new();
        let db = &test_db.db;

        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: Some("{}".to_string()),
            body: None,
            response_status: Some(200),
            response_headers: Some("{}".to_string()),
            response_body: Some(r#"{"users":[]}"#.to_string()),
            response_time_ms: Some(150),
            created_at: Utc::now(),
        };

        db.add_history(&entry).unwrap();

        let history = db.get_history(10, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].method, "GET");
        assert_eq!(history[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_environment_operations() {
        let test_db = TestDb::new();
        let db = &test_db.db;

        let env = Environment {
            id: uuid::Uuid::new_v4().to_string(),
            name: "测试环境".to_string(),
            variables: r#"{"base_url":"https://test.api.com","api_key":"test123"}"#.to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.save_environment(&env).unwrap();

        let envs = db.get_environments().unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "测试环境");

        let active = db.get_active_environment().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().name, "测试环境");
    }

    #[test]
    fn test_search_history() {
        let test_db = TestDb::new();
        let db = &test_db.db;

        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: None,
            body: None,
            response_status: Some(200),
            response_headers: None,
            response_body: None,
            response_time_ms: Some(100),
            created_at: Utc::now(),
        };

        db.add_history(&entry).unwrap();

        let results = db.search_history("users", 10).unwrap();
        assert_eq!(results.len(), 1);

        let results = db.search_history("nonexistent", 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
