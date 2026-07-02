//! 数据库模块
//!
//! 负责SQLite数据库的初始化和数据访问
//! 存储历史记录、环境变量、收藏请求等数据

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// 历史记录最大保留条数，超出后自动清理最旧的记录
const MAX_HISTORY_ENTRIES: usize = 1000;

/// 数据库管理器
pub struct Database {
    /// 数据库连接
    conn: Mutex<Connection>,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new(path: &str) -> Result<Self> {
        let path = Self::expand_path(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建数据库目录: {}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("无法打开数据库: {}", path.display()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_tables()?;
        Ok(db)
    }

    /// 展开路径中的~符号（跨平台）
    fn expand_path(path: &str) -> PathBuf {
        if let Some(stripped) = path.strip_prefix("~/") {
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .or_else(|| {
                    std::env::var_os("HOMEDRIVE")
                        .and_then(|d| {
                            std::env::var_os("HOMEPATH")
                                .map(|p| format!("{}{}", d.to_string_lossy(), p.to_string_lossy()).into())
                        })
                });
            if let Some(home) = home {
                return PathBuf::from(home).join(stripped);
            }
        }
        PathBuf::from(path)
    }

    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒，无法获取连接"))?;

        // 启用 WAL 模式以允许读写并发
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA synchronous=NORMAL;"
        )?;

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
                response_size INTEGER,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        // 兼容旧数据库：尝试添加 response_size 列
        let _ = conn.execute("ALTER TABLE history ADD COLUMN response_size INTEGER", []);
        // 兼容旧数据库：尝试添加 is_global 列
        let _ = conn.execute("ALTER TABLE environments ADD COLUMN is_global INTEGER DEFAULT 0", []);

        // 创建环境变量表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS environments (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                variables TEXT NOT NULL,
                is_active INTEGER DEFAULT 0,
                is_global INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建全局变量表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS global_variables (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
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

        // 创建工作区状态表（保存退出前的 tabs、response 等）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workspace_state (
                id INTEGER PRIMARY KEY DEFAULT 1,
                tabs_json TEXT NOT NULL DEFAULT '[]',
                active_tab INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        // 确保只有一行
        conn.execute(
            "INSERT OR IGNORE INTO workspace_state (id, tabs_json, active_tab, updated_at) VALUES (1, '[]', 0, '')",
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
            "INSERT INTO history (id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, response_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                entry.response_size,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        // 超出上限时删除最旧记录
        drop(conn);
        self.prune_history(MAX_HISTORY_ENTRIES)?;
        Ok(())
    }

    /// 删除超出上限的历史记录，仅保留最新的 `keep` 条
    fn prune_history(&self, keep: usize) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM history", [], |r| r.get(0))?;
        if count as usize > keep {
            let delete_count = count as usize - keep;
            conn.execute(
                "DELETE FROM history WHERE id IN (
                    SELECT id FROM history ORDER BY created_at ASC LIMIT ?1
                )",
                params![delete_count],
            )?;
        }
        Ok(())
    }

    /// 获取历史记录列表
    pub fn get_history(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, response_size, created_at
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
                response_size: row.get(9)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
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
            "SELECT id, method, url, headers, body, response_status, response_headers, response_body, response_time_ms, response_size, created_at
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
                response_size: row.get(9)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
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
            "INSERT INTO environments (id, name, variables, is_active, is_global, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                variables = excluded.variables,
                is_active = excluded.is_active,
                is_global = excluded.is_global,
                updated_at = excluded.updated_at",
            params![
                env.id,
                env.name,
                env.variables,
                env.is_active as i32,
                env.is_global as i32,
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
            "SELECT id, name, variables, is_active, is_global, created_at, updated_at FROM environments ORDER BY is_global DESC, name"
        )?;

        let envs = stmt.query_map([], |row| {
            Ok(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                variables: row.get(2)?,
                is_active: row.get::<_, i32>(3)? == 1,
                is_global: row.get::<_, i32>(4)? == 1,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
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
            "SELECT id, name, variables, is_active, is_global, created_at, updated_at FROM environments WHERE is_active = 1 LIMIT 1"
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                variables: row.get(2)?,
                is_active: true,
                is_global: row.get::<_, i32>(4)? == 1,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
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

    /// 保存全局变量（原子替换）
    pub fn save_global_variables(&self, vars: &HashMap<String, String>) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("DELETE FROM global_variables", [])?;
        let mut stmt = conn.prepare(
            "INSERT INTO global_variables (key, value) VALUES (?1, ?2)"
        )?;
        for (k, v) in vars {
            stmt.execute(params![k, v])?;
        }
        Ok(())
    }

    /// 加载所有全局变量
    pub fn get_global_variables(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare("SELECT key, value FROM global_variables")?;
        let map = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.collect::<Result<HashMap<_, _>, _>>()?;
        Ok(map)
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

    /// 按 ID 查询单个收藏请求
    pub fn get_saved_request_by_id(&self, id: &str) -> Result<Option<SavedRequest>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, method, url, headers, body, description, folder_id, created_at, updated_at
             FROM saved_requests WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
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
        })?;
        Ok(rows.next().transpose()?)
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

    // ==================== 文件夹操作 ====================

    pub fn get_folders(&self) -> Result<Vec<Folder>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id, created_at FROM folders ORDER BY name"
        )?;
        let folders = stmt.query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                created_at: row.get::<_, String>(3).ok(),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(folders)
    }

    pub fn save_folder(&self, folder: &Folder) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO folders (id, name, parent_id, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, parent_id = excluded.parent_id",
            params![
                folder.id,
                folder.name,
                folder.parent_id,
                folder.created_at.clone().unwrap_or_default(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_folder(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute("UPDATE saved_requests SET folder_id = NULL WHERE folder_id = ?1", params![id])?;
        conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 创建文件夹
    pub fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<Folder> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO folders (id, name, parent_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, parent_id, now],
        )?;
        Ok(Folder { id, name: name.to_string(), parent_id: parent_id.map(String::from), created_at: Some(now) })
    }

    /// 按文件夹查询请求（None 表示查询未分类的请求）
    pub fn get_requests_by_folder(&self, folder_id: Option<&str>) -> Result<Vec<SavedRequest>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let query = match folder_id {
            Some(_) => "SELECT id, name, method, url, headers, body, description, folder_id, created_at, updated_at FROM saved_requests WHERE folder_id = ?1 ORDER BY updated_at DESC",
            None => "SELECT id, name, method, url, headers, body, description, folder_id, created_at, updated_at FROM saved_requests WHERE folder_id IS NULL ORDER BY updated_at DESC",
        };
        let mut stmt = conn.prepare(query)?;
        let requests = stmt.query_map(params![folder_id], |row| {
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

    /// 移动请求到指定文件夹
    pub fn move_request(&self, request_id: &str, folder_id: Option<&str>) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE saved_requests SET folder_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![folder_id, now, request_id],
        )?;
        Ok(())
    }

    /// 移动文件夹到指定父文件夹
    pub fn move_folder(&self, folder_id: &str, parent_id: Option<&str>) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "UPDATE folders SET parent_id = ?1 WHERE id = ?2",
            params![parent_id, folder_id],
        )?;
        Ok(())
    }

    /// 级联删除文件夹（删除所有子文件夹，子请求移回根目录）
    pub fn delete_folder_cascade(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        // 递归收集所有子孙文件夹 ID
        let all_ids = self.collect_descendant_folder_ids(id)?;
        // 将所有这些文件夹下的请求移回根目录
        for fid in &all_ids {
            conn.execute("UPDATE saved_requests SET folder_id = NULL WHERE folder_id = ?1", params![fid])?;
        }
        // 删除所有子孙文件夹
        for fid in &all_ids {
            conn.execute("DELETE FROM folders WHERE id = ?1", params![fid])?;
        }
        Ok(())
    }

    /// 递归收集文件夹的所有子孙 ID（内部方法，调用方需持有锁）
    fn collect_descendant_ids_impl(
        conn: &Connection,
        parent_id: &str,
        result: &mut Vec<String>,
    ) -> Result<()> {
        let mut stmt = conn.prepare("SELECT id FROM folders WHERE parent_id = ?1")?;
        let child_ids: Vec<String> = stmt.query_map(params![parent_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for child_id in child_ids {
            Self::collect_descendant_ids_impl(conn, &child_id, result)?;
            result.push(child_id);
        }
        Ok(())
    }

    /// 递归收集文件夹的所有子孙 ID
    pub fn collect_descendant_folder_ids(&self, id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let mut result = vec![id.to_string()];
        Self::collect_descendant_ids_impl(&conn, id, &mut result)?;
        Ok(result)
    }

    /// 保存工作区状态（tabs、response 等）
    pub fn save_workspace_state(&self, tabs_json: &str, active_tab: usize) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        conn.execute(
            "UPDATE workspace_state SET tabs_json = ?1, active_tab = ?2, updated_at = ?3 WHERE id = 1",
            params![tabs_json, active_tab as i64, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }


    /// 加载工作区状态
    pub fn load_workspace_state(&self) -> Result<(String, usize)> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("数据库锁中毒"))?;
        let result = conn.query_row(
            "SELECT tabs_json, active_tab FROM workspace_state WHERE id = 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize)),
        )?;
        Ok(result)
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
    pub response_size: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// 文件夹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: Option<String>,
}

/// 环境变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: String,  // JSON格式的键值对
    pub is_active: bool,
    pub is_global: bool,
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
            response_size: None,
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
            is_global: false,
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
            response_size: None,
            created_at: Utc::now(),
        };

        db.add_history(&entry).unwrap();

        let results = db.search_history("users", 10).unwrap();
        assert_eq!(results.len(), 1);

        let results = db.search_history("nonexistent", 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
