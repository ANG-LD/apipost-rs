//! 认证授权模块
//!
//! 支持多种认证类型：No Auth, Bearer Token, Basic Auth, API Key

use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::{IndexPath, Sizable, StyledExt};
use gpui::*;
use std::sync::Arc;

/// 认证类型枚举
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AuthType {
    NoAuth,
    BearerToken,
    BasicAuth,
    ApiKey,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::NoAuth
    }
}

impl AuthType {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => AuthType::NoAuth,
            1 => AuthType::BearerToken,
            2 => AuthType::BasicAuth,
            3 => AuthType::ApiKey,
            _ => AuthType::NoAuth,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            AuthType::NoAuth => 0,
            AuthType::BearerToken => 1,
            AuthType::BasicAuth => 2,
            AuthType::ApiKey => 3,
        }
    }

    pub fn all() -> Vec<gpui::SharedString> {
        vec![
            "No Auth".into(),
            "Bearer Token".into(),
            "Basic Auth".into(),
            "API Key".into(),
        ]
    }
}

/// API Key Location
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ApiKeyLocation {
    Header,
    Query,
}

impl Default for ApiKeyLocation {
    fn default() -> Self {
        ApiKeyLocation::Header
    }
}

impl ApiKeyLocation {
    pub fn all() -> Vec<gpui::SharedString> {
        vec!["Header".into(), "Query".into()]
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ApiKeyLocation::Header,
            1 => ApiKeyLocation::Query,
            _ => ApiKeyLocation::Header,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            ApiKeyLocation::Header => 0,
            ApiKeyLocation::Query => 1,
        }
    }
}

/// API Key 认证数据
#[derive(Clone)]
pub struct ApiKeyAuthData {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub location: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub location_value: ApiKeyLocation,
}

/// Bearer Token 认证数据
#[derive(Clone)]
pub struct BearerTokenAuthData {
    pub token: Entity<InputState>,
}

/// Basic Auth 认证数据
#[derive(Clone)]
pub struct BasicAuthData {
    pub username: Entity<InputState>,
    pub password: Entity<InputState>,
}

/// 认证状态
#[derive(Clone)]
pub enum AuthState {
    NoAuth,
    Bearer(BearerTokenAuthData),
    Basic(BasicAuthData),
    ApiKey(ApiKeyAuthData),
}

impl Default for AuthState {
    fn default() -> Self {
        AuthState::NoAuth
    }
}

impl AuthState {
    /// 将认证信息转换为 HTTP 请求头
    pub fn to_headers(&self, cx: &Context<crate::ui::MainView>) -> Vec<(String, String)> {
        match self {
            AuthState::NoAuth => vec![],
            AuthState::Bearer(auth) => {
                let token = auth.token.read(cx).value().to_string();
                if token.is_empty() {
                    vec![]
                } else {
                    vec![("Authorization".to_string(), format!("Bearer {}", token))]
                }
            }
            AuthState::Basic(auth) => {
                let username = auth.username.read(cx).value().to_string();
                let password = auth.password.read(cx).value().to_string();
                if username.is_empty() {
                    vec![]
                } else {
                    use base64::Engine;
                    let credentials = base64::engine::general_purpose::STANDARD.encode(
                        format!("{}:{}", username, password)
                    );
                    vec![("Authorization".to_string(), format!("Basic {}", credentials))]
                }
            }
            AuthState::ApiKey(auth) => {
                let key = auth.key.read(cx).value().to_string();
                let value = auth.value.read(cx).value().to_string();
                if key.is_empty() || value.is_empty() {
                    vec![]
                } else {
                    vec![(key, value)]
                }
            }
        }
    }

    /// 获取 API Key 的 query 参数（如果 location 是 query）
    pub fn to_query_params(&self, cx: &Context<crate::ui::MainView>) -> Option<(String, String)> {
        match self {
            AuthState::ApiKey(auth) => {
                if auth.location_value == ApiKeyLocation::Query {
                    let key = auth.key.read(cx).value().to_string();
                    let value = auth.value.read(cx).value().to_string();
                    if key.is_empty() || value.is_empty() {
                        None
                    } else {
                        Some((key, value))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 创建认证类型选择器状态
    pub fn create_auth_type_select(_app_state: Arc<std::sync::Mutex<crate::app::AppState>>, window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Entity<SelectState<Vec<gpui::SharedString>>> {
        let auth_types = AuthType::all();
        cx.new(|cx| {
            SelectState::new(auth_types, Some(IndexPath::default()), window, cx)
        })
    }

    /// 切换 API Key 的位置（Header / Query）
    pub fn toggle_api_key_location(&mut self) {
        if let AuthState::ApiKey(auth) = self {
            auth.location_value = match auth.location_value {
                ApiKeyLocation::Header => ApiKeyLocation::Query,
                ApiKeyLocation::Query => ApiKeyLocation::Header,
            };
        }
    }
}

/// 渲染认证类型的标签
pub fn auth_type_label(auth_type: AuthType) -> &'static str {
    match auth_type {
        AuthType::NoAuth => "No Auth",
        AuthType::BearerToken => "Bearer Token",
        AuthType::BasicAuth => "Basic Auth",
        AuthType::ApiKey => "API Key",
    }
}
