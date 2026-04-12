//! 主视图模块
//!
//! 应用主界面，包含侧边栏和请求/响应面板

use crate::app::database::HistoryEntry;
use crate::app::HttpResponse;
use crate::http::HttpRequest;
use crate::app::history::CreateHistoryEntry;
use crate::ui::{
    ApiKeyLocation, AuthState, AuthType, BodyState, BodyType,
    HeaderEntry, RawFormat, RequestSettings, ScriptState,
    SettingsInputs,
};
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::button::Button;
use gpui_component::{Disableable, IndexPath, Sizable, StyledExt};
use std::sync::Arc;
use tokio;

/// HTTP方法颜色
fn method_color(method: &str) -> u32 {
    match method.to_uppercase().as_str() {
        "GET" => 0x22c55e,
        "POST" => 0xf59e0b,
        "PUT" => 0x3b82f6,
        "DELETE" => 0xef4444,
        "PATCH" => 0x8b5cf6,
        "HEAD" => 0x6b7280,
        "OPTIONS" => 0x8b5cf6,
        _ => 0x6b7280,
    }
}

/// 请求标签
#[derive(Clone)]
pub struct RequestTab {
    pub id: usize,
    pub method: String,
    pub url: String,
    pub name: String,
}

/// 请求构造器标签页
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BuilderTab {
    Params,
    Authorization,
    Headers,
    Body,
    PreRequest,
    Tests,
    Settings,
}

/// 侧边栏标签页
#[derive(Clone, Copy, PartialEq)]
pub enum SidebarTab {
    Collections,
    History,
    Environments,
}

/// 响应面板标签
#[derive(Clone, Copy, PartialEq)]
pub enum ResponseTab {
    Body,
    Cookies,
    Headers,
    TestResults,
}

/// URL参数条目
#[derive(Clone)]
pub struct ParamEntry {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub enabled: bool,
}

/// 主视图
pub struct MainView {
    /// 应用状态
    app_state: Arc<crate::app::AppState>,
    /// HTTP方法
    pub method: String,
    /// URL
    pub url: String,
    /// 请求标签列表
    request_tabs: Vec<RequestTab>,
    /// 当前活动的请求标签
    active_tab: usize,
    /// 响应
    response: Option<HttpResponse>,
    /// 响应面板标签
    response_tab: ResponseTab,
    /// 是否正在加载
    is_loading: bool,
    /// 错误消息
    error_message: Option<String>,
    /// 侧边栏是否折叠
    sidebar_collapsed: bool,
    /// 当前侧边栏标签页
    sidebar_tab: SidebarTab,
    /// 历史记录列表
    history: Vec<HistoryEntry>,
    /// URL输入状态
    url_input: Entity<InputState>,
    /// URL输入变化订阅
    _url_change_sub: gpui::Subscription,
    /// HTTP方法选择状态
    method_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    /// 当前请求构造器标签页
    builder_tab: BuilderTab,
    /// URL参数
    params: Vec<ParamEntry>,
    /// Headers 列表
    headers: Vec<HeaderEntry>,
    /// Body 状态
    body_state: BodyState,
    /// Body 类型选择器
    body_type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    /// Raw 格式选择器
    raw_format_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    /// 认证类型选择器
    auth_type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    /// 认证状态
    auth_state: AuthState,
    /// 脚本状态
    script_state: ScriptState,
    /// 设置
    settings: RequestSettings,
    /// 设置输入状态
    settings_inputs: SettingsInputs,
}

impl MainView {
    /// 创建新的主视图
    pub fn new(app_state: Arc<crate::app::AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 加载历史记录
        let history = app_state.db
            .get_history(50, 0)
            .unwrap_or_default();

        // 创建URL输入状态
        let url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("https://httpbin.org/get")
                .placeholder("Enter URL...")
        });

        // 设置URL输入变化订阅（URL变化时自动解析到params）
        let url_input_clone = url_input.clone();
        let _url_change_sub = cx.subscribe_in(&url_input, window, move |this, _state, event, _window, cx| {
            match event {
                InputEvent::Change => {
                    let url = url_input_clone.read(cx).value().to_string();
                    if url.contains('?') {
                        this.parse_url_to_params_internal(&url, _window, cx);
                    } else {
                        // 清空params
                        this.params.clear();
                        cx.notify();
                    }
                }
                _ => {}
            }
        });

        // 创建HTTP方法选择状态
        let methods: Vec<gpui::SharedString> = vec![
            "GET".into(),
            "POST".into(),
            "PUT".into(),
            "DELETE".into(),
            "PATCH".into(),
            "HEAD".into(),
            "OPTIONS".into(),
        ];
        let method_select = cx.new(|cx| {
            SelectState::new(methods, Some(IndexPath::default()), window, cx)
        });

        // 创建参数输入状态（从空开始，支持动态添加）
        let params: Vec<ParamEntry> = vec![];

        // 创建 Headers 列表（从空开始，支持动态添加）
        let headers: Vec<HeaderEntry> = vec![];

        // 创建 Body 状态
        let raw_content = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(r#"{"key": "value"}"#)
        });
        let body_state = BodyState::new(raw_content);

        // 创建 Body 类型选择器
        let body_type_select = BodyState::create_body_type_select(window, cx);

        // 创建 Raw 格式选择器
        let raw_format_select = BodyState::create_raw_format_select(window, cx);

        // 创建 Auth 类型选择器
        let auth_type_select = AuthState::create_auth_type_select(app_state.clone(), window, cx);

        // 创建认证状态（默认 NoAuth）
        let auth_state = AuthState::NoAuth;

        // 创建脚本状态
        let script_state = ScriptState::new(window, cx);

        // 创建设置
        let settings = RequestSettings::default();
        let settings_inputs = SettingsInputs::new(window, cx);

        Self {
            app_state,
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            request_tabs: vec![RequestTab {
                id: 1,
                method: "GET".to_string(),
                url: "https://httpbin.org/get".to_string(),
                name: "New Request".to_string(),
            }],
            active_tab: 0,
            response: None,
            response_tab: ResponseTab::Body,
            is_loading: false,
            error_message: None,
            sidebar_collapsed: false,
            sidebar_tab: SidebarTab::History,
            history,
            url_input,
            _url_change_sub,
            method_select,
            builder_tab: BuilderTab::Params,
            params,
            headers,
            body_state,
            body_type_select,
            raw_format_select,
            auth_type_select,
            auth_state,
            script_state,
            settings,
            settings_inputs,
        }
    }

    /// 发送HTTP请求
    pub fn send_request(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }

        // 设置加载状态
        self.is_loading = true;
        self.error_message = None;

        // 1. 获取认证头
        let auth_headers = self.auth_state.to_headers(cx);

        // 2. 获取用户定义的 Headers
        let user_headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled)
            .filter_map(|h| {
                let key = h.key.read(cx).value().to_string();
                let value = h.value.read(cx).value().to_string();
                if key.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect();

        // 3. 合并 Headers（认证头优先）
        let mut all_headers: Vec<(String, String)> = auth_headers;
        all_headers.extend(user_headers);

        // 4. 获取 Content-Type
        let content_type = self.body_state.content_type();

        // 5. 如果有 content_type 但不在 headers 中，添加它
        if let Some(ref ct) = content_type {
            if !all_headers.iter().any(|(k, _)| k.to_lowercase() == "content-type") {
                all_headers.push(("Content-Type".to_string(), ct.clone()));
            }
        }

        // 6. 构建完整URL（包含参数和 API Key query 参数）
        // 先从URL中提取base部分（不含query string）
        let base_url = if let Some(query_start) = self.url.find('?') {
            self.url[..query_start].to_string()
        } else {
            self.url.clone()
        };

        let params: Vec<(String, String, bool)> = self.params.iter().map(|p| {
            let key = p.key.read(cx).value().to_string();
            let value = p.value.read(cx).value().to_string();
            (key, value, p.enabled)
        }).collect();

        let enabled_params: Vec<&(String, String, bool)> = params.iter().filter(|p| p.2 && !p.0.is_empty()).collect();

        let mut full_url = if enabled_params.is_empty() {
            base_url
        } else {
            let query_string: String = enabled_params
                .iter()
                .map(|(key, value, _)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", base_url, query_string)
        };

        // 7. 如果是 API Key 认证且 location 是 query，添加到 URL
        if let Some((key, value)) = self.auth_state.to_query_params(cx) {
            let encoded_key = urlencoding::encode(&key);
            let encoded_value = urlencoding::encode(&value);
            if full_url.contains('?') {
                full_url = format!("{}&{}={}", full_url, encoded_key, encoded_value);
            } else {
                full_url = format!("{}?{}={}", full_url, encoded_key, encoded_value);
            }
        }

        // 8. 获取 Body
        let body = self.body_state.to_body(cx);

        // 构建历史记录用的 headers 文本（在移动 all_headers 之前）
        let headers_text_for_history: String = all_headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let body_for_history = body.clone();

        let request = HttpRequest {
            method: self.method.clone(),
            url: full_url,
            headers: all_headers,
            body,
            content_type,
        };

        // 使用同步方式发送请求 (简化处理)
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(self.app_state.send_request(request));

        match result {
            Ok(response) => {
                // 保存历史记录
                let history_entry = CreateHistoryEntry {
                    method: self.method.clone(),
                    url: self.url.clone(),
                    headers: Some(headers_text_for_history),
                    body: body_for_history,
                    response_status: Some(response.status as i32),
                    response_headers: Some(serde_json::to_string(&response.headers).unwrap_or_default()),
                    response_body: Some(response.body.clone()),
                    response_time_ms: Some(response.time_ms),
                };

                if let Err(e) = self.app_state.db.add_history(&history_entry.into_history_entry()) {
                    log::error!("保存历史记录失败: {}", e);
                }

                self.response = Some(response);
                // 刷新历史记录
                if let Ok(hist) = self.app_state.db.get_history(50, 0) {
                    self.history = hist;
                }
            }
            Err(e) => {
                self.error_message = Some(e);
            }
        }

        self.is_loading = false;
    }

    /// 从历史记录加载请求
    #[allow(dead_code)]
    pub fn load_from_history(&mut self, entry: &HistoryEntry, cx: &mut Context<Self>) {
        self.method = entry.method.clone();
        self.url = entry.url.clone();
        // TODO: 解析历史记录的 headers 和 body 到新的数据结构
        self.response = None;
        cx.notify();
    }

    /// 切换响应标签
    pub fn set_response_tab(&mut self, tab: ResponseTab, cx: &mut Context<Self>) {
        self.response_tab = tab;
        cx.notify();
    }

    /// 切换请求构造器标签
    pub fn set_builder_tab(&mut self, tab: BuilderTab, cx: &mut Context<Self>) {
        self.builder_tab = tab;
        cx.notify();
    }

    /// 切换侧边栏标签
    #[allow(dead_code)]
    pub fn set_sidebar_tab(&mut self, tab: SidebarTab, cx: &mut Context<Self>) {
        self.sidebar_tab = tab;
        cx.notify();
    }

    /// 切换侧边栏折叠状态
    #[allow(dead_code)]
    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    // ==================== Params 操作 ====================

    /// 添加新的参数行
    pub fn add_param(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
        self.params.push(ParamEntry { key, value, enabled: true });
        cx.notify();
    }

    /// 删除指定索引的参数行
    pub fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.params.len() {
            self.params.remove(index);
            self.sync_params_to_url(cx);
            cx.notify();
        }
    }

    /// 切换参数启用状态
    pub fn toggle_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.params.len() {
            self.params[index].enabled = !self.params[index].enabled;
            self.sync_params_to_url(cx);
            cx.notify();
        }
    }

    // ==================== Headers 操作 ====================

    /// 添加新的 Header 行
    pub fn add_header(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.headers.push(HeaderEntry::new(window, cx));
        cx.notify();
    }

    /// 删除指定索引的 Header 行
    pub fn remove_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.headers.len() {
            self.headers.remove(index);
            cx.notify();
        }
    }

    /// 切换 Header 启用状态
    pub fn toggle_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.headers.len() {
            self.headers[index].enabled = !self.headers[index].enabled;
            cx.notify();
        }
    }

    // ==================== Auth 操作 ====================

    /// 切换认证类型
    pub fn set_auth_type(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let auth_type = AuthType::from_index(index);
        self.auth_state = match auth_type {
            AuthType::NoAuth => AuthState::NoAuth,
            AuthType::BearerToken => {
                let token = cx.new(|cx| InputState::new(window, cx).default_value(""));
                AuthState::Bearer(crate::ui::BearerTokenAuthData { token })
            }
            AuthType::BasicAuth => {
                let username = cx.new(|cx| InputState::new(window, cx).default_value(""));
                let password = cx.new(|cx| InputState::new(window, cx).default_value(""));
                AuthState::Basic(crate::ui::BasicAuthData { username, password })
            }
            AuthType::ApiKey => {
                let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
                let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
                let locations = ApiKeyLocation::all();
                let location = cx.new(|cx| SelectState::new(locations, Some(IndexPath::default()), window, cx));
                AuthState::ApiKey(crate::ui::ApiKeyAuthData { key, value, location, location_value: ApiKeyLocation::Header })
            }
        };
        cx.notify();
    }

    /// 获取当前认证类型
    pub fn get_auth_type(&self) -> AuthType {
        match &self.auth_state {
            AuthState::NoAuth => AuthType::NoAuth,
            AuthState::Bearer(_) => AuthType::BearerToken,
            AuthState::Basic(_) => AuthType::BasicAuth,
            AuthState::ApiKey(_) => AuthType::ApiKey,
        }
    }

    // ==================== Body 操作 ====================

    /// 设置 Body 类型
    pub fn set_body_type(&mut self, index: usize, cx: &mut Context<Self>) {
        self.body_state.body_type = BodyType::from_index(index);
        cx.notify();
    }

    /// 获取当前 Body 类型
    pub fn get_body_type(&self) -> BodyType {
        self.body_state.body_type
    }

    /// 设置 Raw 格式
    pub fn set_raw_format(&mut self, index: usize, cx: &mut Context<Self>) {
        self.body_state.raw_format = RawFormat::from_index(index);
        cx.notify();
    }

    /// 获取当前 Raw 格式
    pub fn get_raw_format(&self) -> RawFormat {
        self.body_state.raw_format
    }

    // ==================== URL 与 Params 同步 ====================

    /// 从URL解析query string并填充到params
    fn parse_url_to_params_internal(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        // 清空现有params
        self.params.clear();

        // 解析URL中的query string
        if let Some(query_start) = url.find('?') {
            let query_string = &url[query_start + 1..];

            for pair in query_string.split('&') {
                if pair.is_empty() {
                    continue;
                }
                if let Some(eq_pos) = pair.find('=') {
                    let key = &pair[..eq_pos];
                    let value = &pair[eq_pos + 1..];
                    let decoded_key = urlencoding::decode(key).map(|s| s.to_string()).unwrap_or_else(|_| key.to_string());
                    let decoded_value = urlencoding::decode(value).map(|s| s.to_string()).unwrap_or_else(|_| value.to_string());

                    let key_entity = cx.new(|cx| InputState::new(window, cx).default_value(&decoded_key));
                    let value_entity = cx.new(|cx| InputState::new(window, cx).default_value(&decoded_value));

                    self.params.push(ParamEntry {
                        key: key_entity,
                        value: value_entity,
                        enabled: true,
                    });
                } else {
                    let decoded_key = urlencoding::decode(pair).map(|s| s.to_string()).unwrap_or_else(|_| pair.to_string());
                    let key_entity = cx.new(|cx| InputState::new(window, cx).default_value(&decoded_key));
                    let value_entity = cx.new(|cx| InputState::new(window, cx).default_value(""));

                    self.params.push(ParamEntry {
                        key: key_entity,
                        value: value_entity,
                        enabled: true,
                    });
                }
            }
        }

        cx.notify();
    }

    /// 更新URL以反映当前的params
    pub fn sync_params_to_url(&mut self, cx: &mut Context<Self>) {
        // 从URL输入框获取当前base URL
        let current_url = self.url_input.read(cx).value().to_string();
        let base_url = if let Some(query_start) = current_url.find('?') {
            current_url[..query_start].to_string()
        } else {
            current_url
        };

        // 构建新的query string
        let params: Vec<(String, String, bool)> = self.params.iter().map(|p| {
            let key = p.key.read(cx).value().to_string();
            let value = p.value.read(cx).value().to_string();
            (key, value, p.enabled)
        }).collect();

        let enabled_params: Vec<&(String, String, bool)> = params.iter().filter(|p| p.2 && !p.0.is_empty()).collect();

        if enabled_params.is_empty() {
            // 没有参数，使用base URL
            self.url = base_url;
        } else {
            let query_string: String = enabled_params
                .iter()
                .map(|(key, value, _)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
                .collect::<Vec<_>>()
                .join("&");
            self.url = format!("{}?{}", base_url, query_string);
        }

        cx.notify();
    }
}

// ====== 构建器标签页按钮宏 ======
macro_rules! builder_tab_button {
    ($cx:expr, $label:expr, $tab:expr, $builder_tab:expr, $id:expr) => {{
        let is_active = $builder_tab == $tab;
        div()
            .id($id)
            .px_4()
            .py_2()
            .text_sm()
            .cursor_pointer()
            .text_color(if is_active { rgb(0xffffff) } else { rgb(0x888888) })
            .bg(if is_active { rgb(0x2d2d2d) } else { rgb(0x1e1e1e) })
            .on_click($cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_builder_tab($tab, cx);
            }))
            .child($label)
    }};
}

// ====== 响应标签页按钮宏 ======
macro_rules! response_tab_button {
    ($cx:expr, $label:expr, $tab:expr, $response_tab:expr, $id:expr) => {{
        let is_active = $response_tab == $tab;
        div()
            .id($id)
            .text_sm()
            .cursor_pointer()
            .text_color(if is_active { rgb(0xffffff) } else { rgb(0x888888) })
            .on_click($cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_response_tab($tab, cx);
            }))
            .child($label)
    }};
}

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_width = if self.sidebar_collapsed { px(48.0) } else { px(280.0) };
        let is_loading = self.is_loading;
        let response = self.response.clone();
        let error_message = self.error_message.clone();
        let history = self.history.clone();
        let method = self.method.clone();
        let params = self.params.clone();
        let headers = self.headers.clone();
        let body_state = self.body_state.clone();
        let auth_state = self.auth_state.clone();
        let auth_type = self.get_auth_type();
        let response_tab = self.response_tab;
        let builder_tab = self.builder_tab;
        let sidebar_tab = self.sidebar_tab;
        let settings = self.settings.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .children([
                // ==================== 主体布局 ====================
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .children([
                        // ==================== 侧边栏 ====================
                        div()
                            .w(sidebar_width)
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(rgb(0x1e1e1e))
                            .border_r(px(1.0))
                            .border_color(rgb(0x333333))
                            .children([
                                // Logo区域
                                div()
                                    .h(px(48.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .border_b(px(1.0))
                                    .border_color(rgb(0x333333))
                                    .child(div().text_color(rgb(0xf97316)).font_semibold().child("ApiPost")),
                                // 标签页按钮
                                div()
                                    .flex()
                                    .flex_row()
                                    .h(px(40.0))
                                    .children([
                                        div()
                                            .id("sidebar-collections")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Collections { rgb(0x3b3b3b) } else { rgb(0x2a2a2a) })
                                            .text_xs()
                                            .text_color(if sidebar_tab == SidebarTab::Collections { rgb(0xffffff) } else { rgb(0x888888) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Collections, cx);
                                            }))
                                            .child("C"),
                                        div()
                                            .id("sidebar-history")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::History { rgb(0x3b3b3b) } else { rgb(0x2a2a2a) })
                                            .text_xs()
                                            .text_color(if sidebar_tab == SidebarTab::History { rgb(0xffffff) } else { rgb(0x888888) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::History, cx);
                                            }))
                                            .child("H"),
                                        div()
                                            .id("sidebar-environments")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Environments { rgb(0x3b3b3b) } else { rgb(0x2a2a2a) })
                                            .text_xs()
                                            .text_color(if sidebar_tab == SidebarTab::Environments { rgb(0xffffff) } else { rgb(0x888888) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Environments, cx);
                                            }))
                                            .child("E"),
                                    ]),
                                // 侧边栏内容
                                if !self.sidebar_collapsed {
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .overflow_y_hidden()
                                        .children([
                                            if sidebar_tab == SidebarTab::History {
                                                if history.is_empty() {
                                                    div().p_4().text_sm().text_color(rgb(0x666666))
                                                        .child("No history yet")
                                                } else {
                                                    div()
                                                        .flex_col()
                                                        .gap_1()
                                                        .p_2()
                                                        .overflow_y_hidden()
                                                        .children(history.iter().map(|entry| {
                                                            let method_clr = method_color(&entry.method);
                                                            let entry_url = entry.url.clone();
                                                            let entry_method = entry.method.clone();
                                                            div()
                                                                .flex_col()
                                                                .gap_1()
                                                                .p_2()
                                                                .rounded_md()
                                                                .cursor_pointer()
                                                                .hover(|s| s.bg(rgb(0x2d2d2d)))
                                                                .bg(rgb(0x252525))
                                                                .children([
                                                                    div().flex().items_center().gap_2().children([
                                                                        div().px_1().py_px().rounded_sm().bg(rgb(method_clr))
                                                                            .text_xs().text_color(rgb(0xffffff))
                                                                            .child(entry_method.clone()),
                                                                        div().flex_1().text_ellipsis().text_xs().text_color(rgb(0xe0e0e0))
                                                                            .child(entry_url.clone()),
                                                                    ]),
                                                                    if let Some(status) = entry.response_status {
                                                                        let status_color = if (200..300).contains(&status) { 0x22c55e } else { 0xef4444 };
                                                                        div().text_xs().text_color(rgb(status_color))
                                                                            .child(format!("{} ({})", status, entry.response_time_ms.map(|t| format!("{}ms", t)).unwrap_or_default()))
                                                                    } else {
                                                                        div()
                                                                    },
                                                                ])
                                                        }))
                                                }
                                            } else if sidebar_tab == SidebarTab::Collections {
                                                div().p_2().text_sm().text_color(rgb(0xa0a0a0)).child("Collection 1")
                                            } else {
                                                div().p_2().text_sm().text_color(rgb(0xa0a0a0)).child("Environments")
                                            },
                                        ])
                                } else {
                                    div().flex_1()
                                },
                                // 折叠/展开按钮
                                div()
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x2d2d2d)))
                                    .border_t(px(1.0))
                                    .border_color(rgb(0x333333))
                                    .text_color(rgb(0x666666))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        this.toggle_sidebar(cx);
                                    }))
                                    .child(if self.sidebar_collapsed { ">" } else { "<" }),
                            ]),
                        // ==================== 主工作区 ====================
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .children([
                                // 请求标签栏
                                div()
                                    .h(px(40.0))
                                    .flex()
                                    .flex_row()
                                    .bg(rgb(0x2d2d2d))
                                    .border_b(px(1.0))
                                    .border_color(rgb(0x333333))
                                    .children(self.request_tabs.iter().enumerate().map(|(i, tab)| {
                                        let is_active = i == self.active_tab;
                                        let method_clr = method_color(&tab.method);
                                        div()
                                            .h(px(40.0))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x333333)))
                                            .bg(if is_active { rgb(0x1e1e1e) } else { rgb(0x2d2d2d) })
                                            .border_b_2()
                                            .border_b(if is_active { px(2.0) } else { px(0.0) })
                                            .border_color(if is_active { rgb(0x3b82f6) } else { rgb(0x333333) })
                                            .text_xs()
                                            .children([
                                                div().px_1().py_px().rounded_sm().bg(rgb(method_clr))
                                                    .text_xs().text_color(rgb(0xffffff))
                                                    .child(tab.method.clone()),
                                                div().text_color(if is_active { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                                                    .text_ellipsis().w(px(120.0))
                                                    .child(tab.name.clone()),
                                                div().text_xs().text_color(rgb(0x666666)).child("x"),
                                            ])
                                    }))
                                    .child(
                                        div().h(px(40.0)).px_3().flex().items_center()
                                            .text_color(rgb(0x888888)).cursor_pointer().child("+"),
                                    ),
                                // 请求构造器
                                div()
                                    .flex()
                                    .w_full() 
                                    .flex_col()
                                    .bg(rgb(0x1e1e1e))
                                    .children([
                                        // 方法和URL行
                                        div()
                                            .w_full()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .overflow_hidden()
                                            .gap(px(8.0))  
                                            .px_2()
                                            .py_2()
                                            .children([
                                                // 方法选择器
                                                div()
                                                    .h(px(34.0))
                                                    .w(px(95.0))
                                                    .flex()
                                                    .child(
                                                        Select::new(&self.method_select)
                                                        .small()
                                                        .h(px(34.0))
                                                        .border_1()
                                                        .border_color(rgb(0x555555))
                                                        .rounded_sm()
                                                        .text_color(rgb(method_color(&method)))
                                                        .font_semibold()
                                                        .flex_none(),
                                                ),
                                                // URL输入框容器
                                                div()
                                                    .flex_1() //占据剩余空间
                                                    .w_full()
                                                    .h(px(34.0))
                                                    .flex()
                                                    .child(
                                                        Input::new(&self.url_input)
                                                            .h(px(34.0))
                                                            .w_full()
                                                            .bg(rgb(0x2d2d2d))
                                                            .border_1()
                                                            .border_color(rgb(0x555555))
                                                            .rounded_sm()
                                                            .text_color(rgb(0xe0e0e0))
                                                    ),
                                                // 发送按钮
                                                div()
                                                    .w_full()
                                                    .flex()
                                                    .h(px(34.0))
                                                    .w(px(95.0))
                                                    .child(
                                                        Button::new("send")
                                                        .large()
                                                        .px_6()
                                                        .rounded_sm()
                                                        .bg(if is_loading { rgb(0x666666) } else { rgb(0x3b82f6) })
                                                        .text_color(rgb(0xffffff))
                                                        .font_semibold()
                                                        .label(if is_loading { "..." } else { "Send" })
                                                        .disabled(is_loading)
                                                        .flex_none()
                                                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                            let url = this.url_input.read(cx).value().to_string();
                                                            let method = this.method_select.read(cx).selected_value()
                                                                .unwrap_or(&gpui::SharedString::from("GET")).clone();
                                                            this.url = url;
                                                            this.method = method.to_string();
                                                            this.send_request(cx);
                                                        })),
                                                    ),
                                            ]),
                                        // 标签页（全部可点击）
                                        div()
                                            .flex()
                                            .flex_row()
                                            .border_b(px(1.0))
                                            .border_color(rgb(0x333333))
                                            .children([
                                                builder_tab_button!(cx, "Params", BuilderTab::Params, builder_tab, "builder-params"),
                                                builder_tab_button!(cx, "Authorization", BuilderTab::Authorization, builder_tab, "builder-auth"),
                                                builder_tab_button!(cx, "Headers", BuilderTab::Headers, builder_tab, "builder-headers"),
                                                builder_tab_button!(cx, "Body", BuilderTab::Body, builder_tab, "builder-body"),
                                                builder_tab_button!(cx, "Pre-request", BuilderTab::PreRequest, builder_tab, "builder-pre-request"),
                                                builder_tab_button!(cx, "Tests", BuilderTab::Tests, builder_tab, "builder-tests"),
                                                builder_tab_button!(cx, "Settings", BuilderTab::Settings, builder_tab, "builder-settings"),
                                            ]),
                                        // 各标签页内容
                                        if builder_tab == BuilderTab::Params {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_2()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    // 表头
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .gap_2()
                                                        .mb_1()
                                                        .children([
                                                            div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                            div().flex_1().text_xs().text_color(rgb(0x888888)).child("Key"),
                                                            div().flex_1().text_xs().text_color(rgb(0x888888)).child("Value"),
                                                            div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                        ]),
                                                    // 参数行
                                                    div()
                                                        .flex_col()
                                                        .gap_2()
                                                        .py_1()
                                                        .children(params.iter().map(|param| {
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_2()
                                                                .items_center()
                                                                .children([
                                                                    div().w(px(30.0)).text_sm().text_color(rgb(0x666666)).child(if param.enabled { "✓" } else { "○" }),
                                                                    div().flex_1()
                                                                        .child(
                                                                            Input::new(&param.key)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().flex_1()
                                                                        .child(
                                                                            Input::new(&param.value)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().w(px(30.0)).text_sm().text_color(rgb(0xef4444)).child("×"),
                                                                ])
                                                        })),
                                                    // 添加行按钮
                                                    div()
                                                        .mt_2()
                                                        .px_1()
                                                        .py_1()
                                                        .child(
                                                            Button::new("add-param")
                                                                .px_1()
                                                                .py_1()
                                                                .text_sm()
                                                                .text_color(rgb(0x3b82f6))
                                                                .bg(rgb(0x1e1e1e))
                                                                .label("+ Add parameter")
                                                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    this.add_param(_window, cx);
                                                                }))
                                                        ),
                                                ])
                                        } else if builder_tab == BuilderTab::Headers {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_2()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    // 表头
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .gap_2()
                                                        .mb_1()
                                                        .children([
                                                            div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                            div().flex_1().text_xs().text_color(rgb(0x888888)).child("Key"),
                                                            div().flex_1().text_xs().text_color(rgb(0x888888)).child("Value"),
                                                            div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                        ]),
                                                    // Header 行
                                                    div()
                                                        .flex_col()
                                                        .gap_2()
                                                        .py_1()
                                                        .children(headers.iter().map(|header| {
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_2()
                                                                .items_center()
                                                                .children([
                                                                    div().w(px(30.0)).text_sm().text_color(rgb(0x666666)).child(if header.enabled { "✓" } else { "○" }),
                                                                    div().flex_1()
                                                                        .child(
                                                                            Input::new(&header.key)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().flex_1()
                                                                        .child(
                                                                            Input::new(&header.value)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().w(px(30.0)).text_sm().text_color(rgb(0xef4444)).child("×"),
                                                                ])
                                                        })),
                                                    // 添加行按钮
                                                    div()
                                                        .mt_2()
                                                        .px_1()
                                                        .py_1()
                                                        .child(
                                                            Button::new("add-header")
                                                                .px_1()
                                                                .py_1()
                                                                .text_sm()
                                                                .text_color(rgb(0x3b82f6))
                                                                .bg(rgb(0x1e1e1e))
                                                                .label("+ Add Header")
                                                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    this.add_header(_window, cx);
                                                                }))
                                                        ),
                                                ])
                                        } else if builder_tab == BuilderTab::Body {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_3()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    // Body 类型选择
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_4()
                                                        .children([
                                                            div().text_sm().text_color(rgb(0x888888)).child("Body:"),
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .children([
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if body_state.body_type == BodyType::None { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("none"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if body_state.body_type == BodyType::FormData { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("form-data"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if body_state.body_type == BodyType::UrlEncoded { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("x-www-form-urlencoded"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if body_state.body_type == BodyType::Raw { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("raw"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if body_state.body_type == BodyType::Binary { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("binary"),
                                                                ]),
                                                        ]),
                                                    // Body 内容
                                                    if body_state.body_type == BodyType::Raw {
                                                        div()
                                                            .flex_col()
                                                            .flex_1()
                                                            .gap_2()
                                                            .children([
                                                                // Raw 格式选择
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .children([
                                                                        div().text_sm().text_color(rgb(0x888888)).child("Format:"),
                                                                        div()
                                                                            .flex()
                                                                            .items_center()
                                                                            .gap_2()
                                                                            .children([
                                                                                div()
                                                                                    .text_sm()
                                                                                    .text_color(if body_state.raw_format == RawFormat::Json { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .child("JSON"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .text_color(if body_state.raw_format == RawFormat::Xml { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .child("XML"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .text_color(if body_state.raw_format == RawFormat::Text { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .child("Text"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .text_color(if body_state.raw_format == RawFormat::Html { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .child("HTML"),
                                                                            ]),
                                                                    ]),
                                                                // Raw 内容编辑器
                                                                div()
                                                                    .flex_1()
                                                                    .bg(rgb(0x2d2d2d))
                                                                    .border_1()
                                                                    .border_color(rgb(0x444444))
                                                                    .rounded_md()
                                                                    .overflow_y_hidden()
                                                                    .child(
                                                                        Input::new(&body_state.raw_content)
                                                                            .flex_1()
                                                                            .min_h(px(200.0))
                                                                            .bg(rgb(0x2d2d2d))
                                                                            .text_color(rgb(0xe0e0e0))
                                                                            .font_family("monospace"),
                                                                    ),
                                                            ])
                                                    } else if body_state.body_type == BodyType::Binary {
                                                        div()
                                                            .flex_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .bg(rgb(0x252525))
                                                            .border_1()
                                                            .border_color(rgb(0x444444))
                                                            .rounded_md()
                                                            .text_sm()
                                                            .text_color(rgb(0x666666))
                                                            .child("Binary content not supported yet")
                                                    } else if body_state.body_type == BodyType::FormData || body_state.body_type == BodyType::UrlEncoded {
                                                        div()
                                                            .flex_1()
                                                            .bg(rgb(0x2d2d2d))
                                                            .border_1()
                                                            .border_color(rgb(0x444444))
                                                            .rounded_md()
                                                            .overflow_y_hidden()
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .min_h(px(200.0))
                                                                    .bg(rgb(0x2d2d2d))
                                                                    .child(
                                                                        Input::new(&body_state.raw_content)
                                                                            .flex_1()
                                                                            .h(px(200.0))
                                                                            .bg(rgb(0x2d2d2d))
                                                                            .text_color(rgb(0xe0e0e0))
                                                                            .font_family("monospace")
                                                                    )
                                                            )
                                                    } else {
                                                        div().flex_1()
                                                    },
                                                ])
                                        } else if builder_tab == BuilderTab::Authorization {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_4()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    // Auth 类型选择
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_3()
                                                        .children([
                                                            div().text_sm().text_color(rgb(0x888888)).child("Type:"),
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .children([
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if auth_type == AuthType::NoAuth { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("No Auth"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if auth_type == AuthType::BearerToken { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("Bearer Token"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if auth_type == AuthType::BasicAuth { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("Basic Auth"),
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(if auth_type == AuthType::ApiKey { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .child("API Key"),
                                                                ]),
                                                        ]),
                                                    // Auth 内容
                                                    match &auth_state {
                                                        AuthState::NoAuth => {
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .flex_1()
                                                                .text_sm()
                                                                .text_color(rgb(0x666666))
                                                                .child("This request does not use any authorization.")
                                                        }
                                                        AuthState::Bearer(auth) => {
                                                            div()
                                                                .flex_col()
                                                                .gap_3()
                                                                .flex_1()
                                                                .children([
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Token"),
                                                                    div()
                                                                        .child(
                                                                            Input::new(&auth.token)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .w(px(400.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                ])
                                                        }
                                                        AuthState::Basic(auth) => {
                                                            div()
                                                                .flex_col()
                                                                .gap_3()
                                                                .flex_1()
                                                                .children([
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Username"),
                                                                    div()
                                                                        .child(
                                                                            Input::new(&auth.username)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .w(px(400.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Password"),
                                                                    div()
                                                                        .child(
                                                                            Input::new(&auth.password)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .w(px(400.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                ])
                                                        }
                                                        AuthState::ApiKey(auth) => {
                                                            div()
                                                                .flex_col()
                                                                .gap_3()
                                                                .flex_1()
                                                                .children([
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Key"),
                                                                    div()
                                                                        .child(
                                                                            Input::new(&auth.key)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .w(px(400.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Value"),
                                                                    div()
                                                                        .child(
                                                                            Input::new(&auth.value)
                                                                                .small()
                                                                                .h(px(32.0))
                                                                                .w(px(400.0))
                                                                                .bg(rgb(0x2d2d2d))
                                                                                .border_1()
                                                                                .border_color(rgb(0x444444))
                                                                                .text_color(rgb(0xe0e0e0)),
                                                                        ),
                                                                    div().text_sm().text_color(rgb(0x888888)).child("Add to"),
                                                                    div()
                                                                        .flex()
                                                                        .items_center()
                                                                        .gap_2()
                                                                        .children([
                                                                            div()
                                                                                .text_sm()
                                                                                .text_color(if auth.location_value == ApiKeyLocation::Header { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                .child("Header"),
                                                                            div()
                                                                                .text_sm()
                                                                                .text_color(if auth.location_value == ApiKeyLocation::Query { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                .child("Query"),
                                                                        ]),
                                                                ])
                                                        }
                                                    },
                                                ])
                                        } else if builder_tab == BuilderTab::PreRequest {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_2()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x888888))
                                                        .child("Pre-request Script (JavaScript) - Runs before the request is sent"),
                                                    div()
                                                        .flex_1()
                                                        .bg(rgb(0x252525))
                                                        .border_1()
                                                        .border_color(rgb(0x444444))
                                                        .rounded_md()
                                                        .overflow_y_hidden()
                                                        .child(
                                                            Input::new(&self.script_state.pre_request_script)
                                                                .flex_1()
                                                                .min_h(px(200.0))
                                                                .bg(rgb(0x252525))
                                                                .text_color(rgb(0xe0e0e0))
                                                                .font_family("monospace"),
                                                        ),
                                                ])
                                        } else if builder_tab == BuilderTab::Tests {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_2()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x888888))
                                                        .child("Test Script (JavaScript) - Runs after the response is received"),
                                                    div()
                                                        .flex_1()
                                                        .bg(rgb(0x252525))
                                                        .border_1()
                                                        .border_color(rgb(0x444444))
                                                        .rounded_md()
                                                        .overflow_y_hidden()
                                                        .child(
                                                            Input::new(&self.script_state.test_script)
                                                                .flex_1()
                                                                .min_h(px(200.0))
                                                                .bg(rgb(0x252525))
                                                                .text_color(rgb(0xe0e0e0))
                                                                .font_family("monospace"),
                                                        ),
                                                ])
                                        } else if builder_tab == BuilderTab::Settings {
                                            div()
                                                .flex_col()
                                                .flex_1()
                                                .gap_4()
                                                .p_3()
                                                .overflow_hidden()
                                                .children([
                                                    // 超时设置
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_3()
                                                        .children([
                                                            div().w(px(140.0)).text_sm().text_color(rgb(0xe0e0e0)).child("Timeout (s):"),
                                                            div()
                                                                .h(px(32.0))
                                                                .w(px(80.0))
                                                                .bg(rgb(0x2d2d2d))
                                                                .border_1()
                                                                .border_color(rgb(0x444444))
                                                                .child(
                                                                    Input::new(&self.settings_inputs.timeout_input)
                                                                        .small()
                                                                        .h(px(30.0))
                                                                        .w(px(76.0))
                                                                        .bg(rgb(0x2d2d2d))
                                                                        .text_color(rgb(0xe0e0e0)),
                                                                ),
                                                        ]),
                                                    // 重试次数
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_3()
                                                        .children([
                                                            div().w(px(140.0)).text_sm().text_color(rgb(0xe0e0e0)).child("Retries:"),
                                                            div()
                                                                .h(px(32.0))
                                                                .w(px(80.0))
                                                                .bg(rgb(0x2d2d2d))
                                                                .border_1()
                                                                .border_color(rgb(0x444444))
                                                                .child(
                                                                    Input::new(&self.settings_inputs.retry_input)
                                                                        .small()
                                                                        .h(px(30.0))
                                                                        .w(px(76.0))
                                                                        .bg(rgb(0x2d2d2d))
                                                                        .text_color(rgb(0xe0e0e0)),
                                                                ),
                                                        ]),
                                                    // 跟随重定向
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_3()
                                                        .children([
                                                            div().w(px(140.0)).text_sm().text_color(rgb(0xe0e0e0)).child("Follow Redirects:"),
                                                            div()
                                                                .text_sm()
                                                                .text_color(if settings.follow_redirects { rgb(0x22c55e) } else { rgb(0x888888) })
                                                                .child(if settings.follow_redirects { "ON" } else { "OFF" }),
                                                        ]),
                                                    // 验证 SSL
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_3()
                                                        .children([
                                                            div().w(px(140.0)).text_sm().text_color(rgb(0xe0e0e0)).child("Verify SSL:"),
                                                            div()
                                                                .text_sm()
                                                                .text_color(if settings.verify_ssl { rgb(0x22c55e) } else { rgb(0x888888) })
                                                                .child(if settings.verify_ssl { "ON" } else { "OFF" }),
                                                        ]),
                                                ])
                                        } else {
                                            div().flex_1().hidden()
                                        },
                                    ]),
                                // 响应查看器
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .bg(rgb(0x1e1e1e))
                                    .children([
                                        // 响应标签页（全部可点击）
                                        div()
                                            .flex()
                                            .flex_row()
                                            .h(px(36.0))
                                            .px_3()
                                            .items_center()
                                            .gap_4()
                                            .border_b(px(1.0))
                                            .border_color(rgb(0x333333))
                                            .children([
                                                response_tab_button!(cx, "Body", ResponseTab::Body, response_tab, "response-body"),
                                                response_tab_button!(cx, "Cookies", ResponseTab::Cookies, response_tab, "response-cookies"),
                                                response_tab_button!(cx, "Headers", ResponseTab::Headers, response_tab, "response-headers"),
                                                response_tab_button!(cx, "Test Results", ResponseTab::TestResults, response_tab, "response-test-results"),
                                            ]),
                                        // 响应内容区
                                        div()
                                            .flex_1()
                                            .p_4()
                                            .overflow_y_hidden()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .text_sm()
                                            .text_color(rgb(0xa0a0a0))
                                            .children([
                                                if let Some(err) = error_message {
                                                    div()
                                                        .p_3()
                                                        .rounded_md()
                                                        .bg(rgb(0x3f2020))
                                                        .text_color(rgb(0xef4444))
                                                        .child(err)
                                                } else if let Some(resp) = response {
                                                    div()
                                                        .flex_col()
                                                        .gap_2()
                                                        .flex_1()
                                                        .overflow_y_hidden()
                                                        .children([
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_4()
                                                                .children([
                                                                    div()
                                                                        .px_2()
                                                                        .py_px()
                                                                        .rounded_sm()
                                                                        .bg(rgb(if (200..300).contains(&resp.status) { 0x22c55e } else { 0xef4444 }))
                                                                        .text_color(rgb(0xffffff))
                                                                        .child(format!("{} {}", resp.status, resp.status_text())),
                                                                    div().text_color(rgb(0x888888)).child(format!("Time: {}ms", resp.time_ms)),
                                                                    div().text_color(rgb(0x888888)).child(format!("Size: {} bytes", resp.size_bytes)),
                                                                ]),
                                                            div()
                                                                .flex_1()
                                                                .mt_2()
                                                                .p_3()
                                                                .rounded_md()
                                                                .bg(rgb(0x252525))
                                                                .font_family("monospace")
                                                                .text_size(px(12.0))
                                                                .text_color(rgb(0xe0e0e0))
                                                                .overflow_y_hidden()
                                                                .child(resp.format_body()),
                                                        ])
                                                } else {
                                                    div()
                                                        .flex_1()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_color(rgb(0x666666))
                                                        .child("Click Send to request")
                                                },
                                            ]),
                                    ]),
                            ]),
                    ]),
                // ==================== 底部状态栏 ====================
                div()
                    .h(px(24.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .bg(rgb(0x252525))
                    .border_t(px(1.0))
                    .border_color(rgb(0x333333))
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .children([
                        div().flex().items_center().gap_4().children([
                            div().child("No Environment"),
                            div().child("*"),
                            div().child("Bearer Token"),
                        ]),
                        div().flex().items_center().gap_4().children([
                            div().child("Online"),
                            div().child("Console"),
                            div().child("Ready"),
                        ]),
                    ]),
            ])
    }
}
