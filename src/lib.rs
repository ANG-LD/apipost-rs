//! ApiPost-Rs 库
//!
//! 提供HTTP客户端、环境变量管理、历史记录等核心功能

pub mod app;
pub mod config;
pub mod http;
pub mod i18n;

pub use app::{AppState, Database, Environment, EnvironmentManager, HistoryEntry, HistoryManager};
pub use config::AppConfig;
pub use http::{generate_code, generate_curl, parse_curl, HttpClient, HttpRequest, HttpResponse};
pub use i18n::I18nManager;
