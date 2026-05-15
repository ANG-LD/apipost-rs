//! 应用模块
//!
//! 包含应用程序的核心业务逻辑

pub mod database;
pub mod environment;
pub mod history;

pub use database::{Database, Environment, HistoryEntry, SavedRequest};
pub use environment::{EnvConfig, EnvVariable, EnvironmentManager};
pub use history::{CreateHistoryEntry, HistoryFilter, HistoryManager};

use crate::config::AppConfig;
use crate::http;
use crate::i18n::I18nManager;

pub use http::{HttpRequest, HttpResponse, HttpClient, RequestOptions};
use gpui::SharedString;
use std::sync::Arc;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// 应用配置
    pub config: AppConfig,
    /// 数据库
    pub db: Arc<Database>,
    /// tokio runtime handle（持久化，避免每次请求重建 runtime）
    pub rt_handle: tokio::runtime::Handle,
    /// 环境变量管理器（与 HttpClient 共享同一实例）
    pub env_manager: Arc<EnvironmentManager>,
    /// HTTP客户端
    pub http_client: HttpClient,
    /// 国际化管理器
    pub i18n: I18nManager,
    /// 主题名称
    pub theme_name: SharedString,
}

impl AppState {
    /// 创建新的应用状态
    ///
    /// # 错误
    /// 返回数据库初始化失败或HTTP客户端创建失败
    pub fn try_new(config: AppConfig, rt_handle: tokio::runtime::Handle) -> anyhow::Result<Self> {
        // 初始化数据库
        let db = Arc::new(Database::new(&config.database.path)
            .map_err(|e| anyhow::anyhow!("数据库初始化失败: {}", e))?);

        // 初始化环境变量管理器（Arc 共享给 HttpClient）
        let env_manager = Arc::new(EnvironmentManager::new());

        // 从数据库加载激活的环境
        if let Ok(Some(env)) = db.get_active_environment() {
            if let Err(e) = env_manager.load_from_json(&env.variables) {
                log::warn!("加载环境变量失败: {}", e);
            }
        }

        // 从数据库加载全局变量
        if let Ok(globals) = db.get_global_variables() {
            if !globals.is_empty() {
                env_manager.set_globals(globals);
            }
        }

        // 初始化HTTP客户端（共享同一个 EnvironmentManager 实例）
        let http_client = match &config.proxy.enabled {
            true => {
                HttpClient::with_proxy(&config.proxy.url, Arc::clone(&env_manager))
                    .unwrap_or_else(|_| {
                        HttpClient::with_timeout(config.general.timeout, Arc::clone(&env_manager))
                            .expect("HTTP客户端初始化失败")
                    })
            }
            false => {
                HttpClient::with_timeout(config.general.timeout, Arc::clone(&env_manager))
                    .unwrap_or_else(|_| {
                        HttpClient::new(Arc::clone(&env_manager))
                            .expect("HTTP客户端初始化失败")
                    })
            }
        };

        // 初始化国际化
        let i18n = I18nManager::new(&config.general.language);

        // 确定主题
        let theme_name: SharedString = config.general.theme.clone().into();

        Ok(Self {
            config,
            db,
            rt_handle,
            env_manager,
            http_client,
            i18n,
            theme_name,
        })
    }

    /// 初始化应用
    pub fn init(&self) {
        log::info!("应用状态初始化完成");
    }

    /// 发送HTTP请求
    pub async fn send_request(&self, request: HttpRequest) -> Result<HttpResponse, String> {
        log::debug!("发送请求: {} {}", request.method, request.url);
        log::debug!("请求头: {:?}", request.headers);
        log::debug!("请求体: {:?}", request.body);
        log::debug!("text_fields: {:?}", request.text_fields);
        log::debug!("file_fields: {:?}", request.file_fields);

        let result = self.http_client.send_request(&request).await;

        match &result {
            Ok(resp) => {
                log::debug!("请求成功: {} - {} ({}ms)", resp.status, resp.status_text(), resp.time_ms);
            }
            Err(err) => {
                log::error!("HTTP请求失败: {:?}", err);
                for (i, e) in err.chain().enumerate() {
                    log::error!("  错误[{}]: {}", i, e);
                }
            }
        }

        result.map_err(|e| {
            e.chain().map(|c| c.to_string()).collect::<Vec<_>>().join(" -> ")
        })
    }

    /// 发送HTTP请求（带设置）
    pub async fn send_request_with_settings(&self, request: HttpRequest, options: RequestOptions) -> Result<HttpResponse, String> {
        self.http_client.send_request_with_settings(&request, options).await.map_err(|e| e.to_string())
    }

    /// 切换主题
    pub fn toggle_theme(&mut self) {
        let current = self.config.general.theme.as_str();
        self.config.general.theme = match current {
            "light" => "dark".to_string(),
            "dark" => "light".to_string(),
            _ => "dark".to_string(),
        };
        self.theme_name = self.config.general.theme.clone().into();
        if let Err(e) = self.config.save() {
            log::error!("保存配置失败: {}", e);
        }
    }

    /// 设置主题
    pub fn set_theme(&mut self, theme: &str) {
        self.config.general.theme = theme.to_string();
        self.theme_name = self.config.general.theme.clone().into();
        if let Err(e) = self.config.save() {
            log::error!("保存配置失败: {}", e);
        }
    }

    /// 切换语言
    pub fn switch_language(&mut self, language: &str) {
        self.config.general.language = language.to_string();
        self.i18n = I18nManager::new(language);
        if let Err(e) = self.config.save() {
            log::error!("保存配置失败: {}", e);
        }
    }

    /// 获取翻译文本
    pub fn t(&self, key: &str) -> String {
        self.i18n.get(key)
    }
}

/// 应用主题
#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn light() -> Self {
        Theme::Light
    }

    pub fn dark() -> Self {
        Theme::Dark
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "dark" | _ => Theme::Dark,
        }
    }
}
