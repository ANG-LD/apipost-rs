//! 历史记录管理模块
//!
//! 负责管理HTTP请求的历史记录
//! 支持历史记录的保存、查询、删除等操作

use crate::app::database::{Database, HistoryEntry};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 历史记录管理器
pub struct HistoryManager {
    /// 数据库引用
    db: Arc<Database>,
}

impl HistoryManager {
    /// 创建新的历史记录管理器
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 添加历史记录
    pub fn add_entry(&self, entry: HistoryEntry) -> Result<()> {
        self.db.add_history(&entry)
    }

    /// 获取历史记录列表
    pub fn get_entries(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        self.db.get_history(limit, offset)
    }

    /// 搜索历史记录
    pub fn search(&self, keyword: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        self.db.search_history(keyword, limit)
    }

    /// 删除单条历史记录
    pub fn delete_entry(&self, id: &str) -> Result<()> {
        self.db.delete_history(id)
    }

    /// 清空所有历史记录
    pub fn clear_all(&self) -> Result<()> {
        self.db.clear_history()
    }

    /// 按HTTP方法过滤
    pub fn filter_by_method(&self, method: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let entries = self.db.get_history(limit, 0)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.method.to_uppercase() == method.to_uppercase())
            .collect())
    }

    /// 按状态码过滤
    pub fn filter_by_status(&self, status: u16, limit: usize) -> Result<Vec<HistoryEntry>> {
        let entries = self.db.get_history(limit, 0)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.response_status == Some(status as i32))
            .collect())
    }

    /// 按时间范围过滤
    pub fn filter_by_timerange(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<HistoryEntry>> {
        let entries = self.db.get_history(1000, 0)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.created_at >= start && e.created_at <= end)
            .collect())
    }
}

/// 创建历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHistoryEntry {
    /// HTTP方法
    pub method: String,
    /// 请求URL
    pub url: String,
    /// 请求头
    pub headers: Option<String>,
    /// 请求体
    pub body: Option<String>,
    /// 响应状态码
    pub response_status: Option<i32>,
    /// 响应头
    pub response_headers: Option<String>,
    /// 响应体
    pub response_body: Option<String>,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<i64>,
    /// 响应体大小（字节，压缩后）
    pub response_size: Option<i64>,
}

impl CreateHistoryEntry {
    /// 转换为数据库历史记录条目
    pub fn into_history_entry(self) -> HistoryEntry {
        HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            method: self.method,
            url: self.url,
            headers: self.headers,
            body: self.body,
            response_status: self.response_status,
            response_headers: self.response_headers,
            response_body: self.response_body,
            response_time_ms: self.response_time_ms,
            response_size: self.response_size,
            created_at: Utc::now(),
        }
    }
}

/// 历史记录过滤条件
#[derive(Debug, Clone, Default)]
pub struct HistoryFilter {
    /// 搜索关键词
    pub keyword: Option<String>,
    /// HTTP方法
    pub method: Option<String>,
    /// 最小状态码
    pub min_status: Option<u16>,
    /// 最大状态码
    pub max_status: Option<u16>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
}

impl HistoryFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置搜索关键词
    pub fn with_keyword(mut self, keyword: String) -> Self {
        self.keyword = Some(keyword);
        self
    }

    /// 设置HTTP方法过滤
    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }

    /// 设置状态码范围
    pub fn with_status_range(mut self, min: u16, max: u16) -> Self {
        self.min_status = Some(min);
        self.max_status = Some(max);
        self
    }

    /// 设置时间范围
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// 应用过滤器到历史记录
    pub fn apply(&self, entries: Vec<HistoryEntry>) -> Vec<HistoryEntry> {
        entries
            .into_iter()
            .filter(|e| {
                // 关键词过滤
                if let Some(ref kw) = self.keyword {
                    let kw_lower = kw.to_lowercase();
                    if !e.url.to_lowercase().contains(&kw_lower)
                        && !e.method.to_lowercase().contains(&kw_lower)
                    {
                        return false;
                    }
                }

                // 方法过滤
                if let Some(ref m) = self.method {
                    if e.method.to_uppercase() != m.to_uppercase() {
                        return false;
                    }
                }

                // 状态码范围过滤
                if let Some(min) = self.min_status {
                    if e.response_status.map(|s| s as u16).unwrap_or(0) < min {
                        return false;
                    }
                }
                if let Some(max) = self.max_status {
                    if e.response_status.map(|s| s as u16).unwrap_or(999) > max {
                        return false;
                    }
                }

                // 时间范围过滤
                if let Some(start) = self.start_time {
                    if e.created_at < start {
                        return false;
                    }
                }
                if let Some(end) = self.end_time {
                    if e.created_at > end {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_history_entry() {
        let entry = CreateHistoryEntry {
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: None,
            body: None,
            response_status: Some(200),
            response_headers: None,
            response_body: Some("{}".to_string()),
            response_time_ms: Some(100),
        };

        let history = entry.into_history_entry();
        assert_eq!(history.method, "GET");
        assert_eq!(history.url, "https://api.example.com");
        assert!(history.response_status == Some(200));
        assert!(!history.id.is_empty());
    }

    #[test]
    fn test_history_filter_keyword() {
        let filter = HistoryFilter::new().with_keyword("example".to_string());

        let entries = vec![
            HistoryEntry {
                id: "1".to_string(),
                method: "GET".to_string(),
                url: "https://api.example.com".to_string(),
                headers: None,
                body: None,
                response_status: Some(200),
                response_headers: None,
                response_body: None,
                response_time_ms: None,
                created_at: Utc::now(),
            },
            HistoryEntry {
                id: "2".to_string(),
                method: "GET".to_string(),
                url: "https://other.com".to_string(),
                headers: None,
                body: None,
                response_status: Some(200),
                response_headers: None,
                response_body: None,
                response_time_ms: None,
                created_at: Utc::now(),
            },
        ];

        let filtered = filter.apply(entries);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn test_history_filter_method() {
        let filter = HistoryFilter::new().with_method("POST".to_string());

        let entries = vec![
            HistoryEntry {
                id: "1".to_string(),
                method: "GET".to_string(),
                url: "https://api.example.com".to_string(),
                headers: None,
                body: None,
                response_status: Some(200),
                response_headers: None,
                response_body: None,
                response_time_ms: None,
                created_at: Utc::now(),
            },
            HistoryEntry {
                id: "2".to_string(),
                method: "POST".to_string(),
                url: "https://api.example.com".to_string(),
                headers: None,
                body: None,
                response_status: Some(201),
                response_headers: None,
                response_body: None,
                response_time_ms: None,
                created_at: Utc::now(),
            },
        ];

        let filtered = filter.apply(entries);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].method, "POST");
    }
}
