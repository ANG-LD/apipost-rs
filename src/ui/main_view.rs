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
    SettingsInputs, Theme,
};
use gpui::prelude::*;
use gpui::*;
use gpui::InteractiveElement;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::button::Button;
use gpui_component::scroll::ScrollableElement;
use gpui_component::scroll::Scrollable;
use gpui_component::{Disableable, Icon, IconName, IndexPath, Sizable, StyledExt};
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

/// 响应体显示模式
#[derive(Clone, Copy, PartialEq)]
pub enum BodyViewMode {
    Pretty,
    Raw,
    Preview,
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
    /// 响应体显示模式
    body_view_mode: BodyViewMode,
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
    /// 是否正在导入cURL（防止URL输入框回写触发循环）
    is_importing_curl: bool,
    /// 响应体输入状态（用于 JSON 语法高亮显示）
    response_input: Entity<InputState>,
    /// 下一个标签页 ID（递增，保证唯一）
    next_tab_id: usize,
    /// Splitter是否正在拖拽
    splitter_dragging: bool,
    /// 拖拽开始时的Y位置
    splitter_start_y: f32,
    /// 请求构造器高度（像素）
    request_builder_height: f32,
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
                    if this.is_importing_curl {
                        return;
                    }
                    let url = url_input_clone.read(cx).value().to_string();
                    // 检测cURL命令
                    if url.trim().starts_with("curl ") || url.trim().starts_with("curl\n") {
                        if let Err(e) = this.import_curl(&url, _window, cx) {
                            log::warn!("解析cURL命令失败: {}", e);
                        }
                    } else if url.contains('?') {
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

        // 创建响应体输入状态（用于 JSON 语法高亮显示）
        let response_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .rows(20)
                .soft_wrap(false)
                .default_value("")
        });

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
            body_view_mode: BodyViewMode::Pretty,
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
            is_importing_curl: false,
            response_input,
            next_tab_id: 2,
            splitter_dragging: false,
            splitter_start_y: 0.0,
            request_builder_height: 400.0,
        }
    }

    /// 发送HTTP请求
    pub fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        // 9. 获取 form-data 字段
        let text_fields = self.body_state.get_form_data_text_fields(cx);
        let file_fields_raw = self.body_state.get_form_data_file_fields(cx);
        let file_fields: Vec<crate::http::FileField> = file_fields_raw
            .into_iter()
            .map(|(field_name, file_path, content_type)| crate::http::FileField {
                field_name,
                file_path,
                content_type,
            })
            .collect();

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
            text_fields,
            file_fields,
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

                self.response = Some(response.clone());
                // 同步更新 response_input 用于显示
                let response_body = response.format_body().to_string();
                self.response_input.update(cx, |state, cx| {
                    state.set_value(&response_body, window, cx);
                });
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

    /// 开始拖拽splitter
    pub fn start_splitter_drag(&mut self, start_y: f32) {
        self.splitter_dragging = true;
        self.splitter_start_y = start_y;
    }

    /// 更新splitter位置（拖拽中）
    pub fn update_splitter_drag(&mut self, current_y: f32) {
        if self.splitter_dragging {
            // 计算delta
            let delta_y = current_y - self.splitter_start_y;
            // 直接调整高度（每像素变化对应相同的像素变化）
            self.request_builder_height = (self.request_builder_height + delta_y).max(100.0);
            self.splitter_start_y = current_y;
        }
    }

    /// 结束拖拽splitter
    pub fn end_splitter_drag(&mut self) {
        self.splitter_dragging = false;
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

    /// 切换 API Key 认证的位置
    pub fn toggle_api_key_location(&mut self, cx: &mut Context<Self>) {
        self.auth_state.toggle_api_key_location();
        cx.notify();
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

    /// 切换跟随重定向设置
    pub fn toggle_follow_redirects(&mut self, cx: &mut Context<Self>) {
        self.settings.follow_redirects = !self.settings.follow_redirects;
        cx.notify();
    }

    /// 切换 SSL 验证设置
    pub fn toggle_verify_ssl(&mut self, cx: &mut Context<Self>) {
        self.settings.verify_ssl = !self.settings.verify_ssl;
        cx.notify();
    }

    /// 添加 form-data 条目
    pub fn add_form_data_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.body_state.add_form_data_entry(window, cx);
    }

    /// 删除 form-data 条目
    pub fn remove_form_data_entry(&mut self, index: usize) {
        self.body_state.remove_form_data_entry(index);
    }

    /// 切换 form-data 条目启用状态
    pub fn toggle_form_data_entry(&mut self, index: usize, cx: &mut Context<Self>) {
        self.body_state.toggle_form_data_entry(index);
        cx.notify();
    }

    /// 设置 form-data 条目类型
    pub fn set_form_data_param_type(&mut self, index: usize, param_type: crate::ui::FormDataParamType, window: &mut Window, cx: &mut Context<Self>) {
        self.body_state.set_form_data_param_type(index, param_type, window, cx);
        cx.notify();
    }

    /// 为 form-data File 类型选择文件
    pub fn pick_file_for_form_data(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        use rfd::FileDialog;

        if index >= self.body_state.form_data.len() {
            return;
        }

        let entry = &self.body_state.form_data[index];
        if entry.param_type != crate::ui::FormDataParamType::File {
            return;
        }

        // 打开文件选择对话框
        if let Some(file_path) = FileDialog::new()
            .pick_file()
        {
            let path_str = file_path.to_string_lossy().to_string();
            self.body_state.update_form_data_file_path(index, &path_str, window, cx);
            cx.notify();
        }
    }

    /// 设置 url-encoded 条目类型
    pub fn set_urlencoded_param_type(&mut self, index: usize, param_type: crate::ui::FormDataParamType, window: &mut Window, cx: &mut Context<Self>) {
        self.body_state.set_urlencoded_param_type(index, param_type, window, cx);
        cx.notify();
    }

    /// 添加 url-encoded 条目
    pub fn add_urlencoded_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.body_state.add_urlencoded_entry(window, cx);
    }

    /// 删除 url-encoded 条目
    pub fn remove_urlencoded_entry(&mut self, index: usize) {
        self.body_state.remove_urlencoded_entry(index);
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

    /// 导入cURL命令并解析为请求参数
    pub fn import_curl(&mut self, curl_command: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let request = crate::http::parse_curl(curl_command).map_err(|e| e.to_string())?;

        self.is_importing_curl = true;

        // 设置HTTP方法
        self.method = request.method.clone();

        // 同步更新 method_select 控件
        let method_idx = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
            .iter()
            .position(|&m| m == request.method.to_uppercase().as_str())
            .unwrap_or(0);
        let idx_path = Some(IndexPath::new(method_idx));
        self.method_select.update(cx, |state, cx| {
            state.set_selected_index(idx_path, window, cx);
        });

        // 分离URL中的query参数和base URL
        let (base_url, query_string) = if let Some(query_pos) = request.url.find('?') {
            (&request.url[..query_pos], Some(&request.url[query_pos + 1..]))
        } else {
            (request.url.as_str(), None)
        };

        // 回写解析后的URL到输入框（替换掉原来的 curl 命令）
        let base_url_str = base_url.to_string();
        self.url = base_url_str.clone();
        self.url_input.update(cx, |state, cx| {
            state.set_value(&base_url_str, window, cx);
        });

        // 清空现有params和headers
        self.params.clear();
        self.headers.clear();

        // 解析query string为params
        if let Some(query) = query_string {
            for pair in query.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let (key, value) = if let Some(eq_pos) = pair.find('=') {
                    let key = urlencoding::decode(&pair[..eq_pos])
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| pair[..eq_pos].to_string());
                    let value = urlencoding::decode(&pair[eq_pos + 1..])
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| pair[eq_pos + 1..].to_string());
                    (key, value)
                } else {
                    (pair.to_string(), String::new())
                };

                let key_entity = cx.new(|cx| InputState::new(window, cx).default_value(&key));
                let value_entity = cx.new(|cx| InputState::new(window, cx).default_value(&value));
                self.params.push(ParamEntry {
                    key: key_entity,
                    value: value_entity,
                    enabled: true,
                });
            }
        }

        // 解析headers
        for (key, value) in request.headers {
            let key_entity = cx.new(|cx| InputState::new(window, cx).default_value(&key));
            let value_entity = cx.new(|cx| InputState::new(window, cx).default_value(&value));
            self.headers.push(HeaderEntry {
                key: key_entity,
                value: value_entity,
                enabled: true,
            });
        }

        // 如果有body，设置到body_state
        if let Some(body) = request.body {
            self.body_state.body_type = crate::ui::BodyType::Raw;
            let body_owned = body;
            self.body_state.raw_content.update(cx, move |this, cx| {
                this.set_value(&body_owned, window, cx);
            });
        }

        self.is_importing_curl = false;
        cx.notify();
        Ok(())
    }

    // ==================== 标签页管理 ====================

    /// 新增请求标签页，重置表单为默认状态
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 保存当前标签的 URL 和方法
        self.save_current_tab_meta(cx);

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.request_tabs.push(RequestTab {
            id,
            method: "GET".to_string(),
            url: String::new(),
            name: "New Request".to_string(),
        });
        let new_idx = self.request_tabs.len() - 1;
        self.active_tab = new_idx;

        // 重置表单
        self.method = "GET".to_string();
        self.url = String::new();
        self.params.clear();
        self.headers.clear();
        self.response = None;
        self.error_message = None;

        self.is_importing_curl = true;
        self.url_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        self.method_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(0)), window, cx);
        });
        self.is_importing_curl = false;

        cx.notify();
    }

    /// 关闭指定标签页（至少保留一个）
    pub fn close_tab(&mut self, tab_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.request_tabs.len() <= 1 {
            return;
        }
        self.request_tabs.remove(tab_idx);

        let new_active = if self.active_tab >= self.request_tabs.len() {
            self.request_tabs.len() - 1
        } else if tab_idx < self.active_tab {
            self.active_tab - 1
        } else {
            self.active_tab.min(self.request_tabs.len() - 1)
        };

        // 直接加载目标标签数据
        self.active_tab = new_active;
        let tab = self.request_tabs[new_active].clone();
        self.load_tab_meta(&tab, window, cx);
    }

    /// 切换到指定标签页
    pub fn switch_tab(&mut self, tab_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if tab_idx == self.active_tab || tab_idx >= self.request_tabs.len() {
            return;
        }
        self.save_current_tab_meta(cx);
        self.active_tab = tab_idx;
        let tab = self.request_tabs[tab_idx].clone();
        self.load_tab_meta(&tab, window, cx);
    }

    /// 将当前表单的 URL / 方法写回当前标签元数据
    fn save_current_tab_meta(&mut self, cx: &mut Context<Self>) {
        if self.active_tab < self.request_tabs.len() {
            let url = self.url_input.read(cx).value().to_string();
            let method = self.method_select.read(cx)
                .selected_value()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "GET".to_string());
            let short = if url.len() > 30 { format!("{}…", &url[..30]) } else { url.clone() };
            self.request_tabs[self.active_tab].url = url;
            self.request_tabs[self.active_tab].method = method;
            self.request_tabs[self.active_tab].name = if short.is_empty() {
                "New Request".to_string()
            } else {
                short
            };
        }
    }

    /// 将标签元数据加载到表单控件
    fn load_tab_meta(&mut self, tab: &RequestTab, window: &mut Window, cx: &mut Context<Self>) {
        self.method = tab.method.clone();
        self.url = tab.url.clone();
        self.params.clear();
        self.response = None;
        self.error_message = None;

        let url = tab.url.clone();
        let method = tab.method.clone();

        let method_idx = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
            .iter()
            .position(|&m| m == method.to_uppercase().as_str())
            .unwrap_or(0);

        self.is_importing_curl = true;
        self.url_input.update(cx, |state, cx| {
            state.set_value(&url, window, cx);
        });
        self.method_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(method_idx)), window, cx);
        });
        self.is_importing_curl = false;

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
        let request_tabs = self.request_tabs.clone();
        let active_tab = self.active_tab;
        let show_close = self.request_tabs.len() > 1;

        // 构建标签页列表
        let tab_items: Vec<Div> = (0..request_tabs.len())
            .map(|i| {
                let tab = &request_tabs[i];
                let is_active = i == active_tab;
                let method_clr = method_color(&tab.method);
                let tab_method = tab.method.clone();
                let tab_name = tab.name.clone();

                div()
                    .h(px(40.0))
                    .pl_3()
                    .pr_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .bg(if is_active { rgb(0x1e1e1e) } else { rgb(0x2d2d2d) })
                    .border_b_2()
                    .border_b(if is_active { px(2.0) } else { px(0.0) })
                    .border_color(if is_active { rgb(0x3b82f6) } else { rgb(0x333333) })
                    .text_xs()
                    .children(if show_close {
                        vec![
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_sm()
                                .px_1()
                                .py_px()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x3a3a3a)))
                                .children([
                                    div().px_1().py_px().rounded_sm()
                                        .bg(rgb(method_clr))
                                        .text_xs().text_color(rgb(0xffffff))
                                        .child(tab_method),
                                    div()
                                        .text_color(if is_active { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                                        .max_w(px(90.0))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(tab_name),
                                ]),
                            div()
                                .w(px(20.0))
                                .h(px(20.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_color(rgb(0x666666))
                                .child(Icon::new(IconName::Close).xsmall()),
                        ]
                    } else {
                        vec![
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_sm()
                                .px_1()
                                .py_px()
                                .children([
                                    div().px_1().py_px().rounded_sm()
                                        .bg(rgb(method_clr))
                                        .text_xs().text_color(rgb(0xffffff))
                                        .child(tab_method),
                                    div()
                                        .text_color(if is_active { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                                        .max_w(px(90.0))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(tab_name),
                                ]),
                        ]
                    })
            })
            .collect();

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
                                            .text_color(if sidebar_tab == SidebarTab::Collections { rgb(0xffffff) } else { rgb(0x666666) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Collections, cx);
                                            }))
                                            .child(Icon::new(IconName::FolderClosed).small()),
                                        div()
                                            .id("sidebar-history")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::History { rgb(0x3b3b3b) } else { rgb(0x2a2a2a) })
                                            .text_color(if sidebar_tab == SidebarTab::History { rgb(0xffffff) } else { rgb(0x666666) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::History, cx);
                                            }))
                                            .child(Icon::new(IconName::GalleryVerticalEnd).small()),
                                        div()
                                            .id("sidebar-environments")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Environments { rgb(0x3b3b3b) } else { rgb(0x2a2a2a) })
                                            .text_color(if sidebar_tab == SidebarTab::Environments { rgb(0xffffff) } else { rgb(0x666666) })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Environments, cx);
                                            }))
                                            .child(Icon::new(IconName::Settings2).small()),
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
                                    .text_color(rgb(0x666666))
                                    .child(if self.sidebar_collapsed {
                                        Icon::new(IconName::PanelLeftOpen).small()
                                    } else {
                                        Icon::new(IconName::PanelLeftClose).small()
                                    }),
                            ]),
                        // ==================== 主工作区 ====================
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                if this.splitter_dragging {
                                    let y: f32 = event.position.y.into();
                                    this.update_splitter_drag(y);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                if this.splitter_dragging {
                                    this.end_splitter_drag();
                                    cx.notify();
                                }
                            }))
                            .children([
                                // 请求标签栏
                                div()
                                    .h(px(40.0))
                                    .flex()
                                    .flex_row()
                                    .bg(rgb(0x2d2d2d))
                                    .border_b(px(1.0))
                                    .border_color(rgb(0x333333))
                                    .children(request_tabs.iter().enumerate().map(|(i, tab)| {
                                        let is_active = i == active_tab;
                                        let method_clr = method_color(&tab.method);
                                        let tab_method = tab.method.clone();
                                        let tab_name = tab.name.clone();
                                        let show_close = show_close;

                                        div()
                                            .h(px(40.0))
                                            .pl_3()
                                            .pr_1()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .bg(if is_active { rgb(0x1e1e1e) } else { rgb(0x2d2d2d) })
                                            .border_b_2()
                                            .border_b(if is_active { px(2.0) } else { px(0.0) })
                                            .border_color(if is_active { rgb(0x3b82f6) } else { rgb(0x333333) })
                                            .text_xs()
                                            .children(if show_close {
                                                vec![
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .rounded_sm()
                                                        .px_1()
                                                        .py_px()
                                                        .cursor_pointer()
                                                        .hover(|s| s.bg(rgb(0x3a3a3a)))
                                                        .children([
                                                            div().px_1().py_px().rounded_sm()
                                                                .bg(rgb(method_clr))
                                                                .text_xs().text_color(rgb(0xffffff))
                                                                .child(tab_method),
                                                            div()
                                                                .text_color(if is_active { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                                                                .max_w(px(90.0))
                                                                .overflow_hidden()
                                                                .text_ellipsis()
                                                                .child(tab_name),
                                                        ]),
                                                    div()
                                                        .w(px(20.0))
                                                        .h(px(20.0))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .rounded_sm()
                                                        .text_color(rgb(0x666666))
                                                        .child(Icon::new(IconName::Close).xsmall()),
                                                ]
                                            } else {
                                                vec![
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .rounded_sm()
                                                        .px_1()
                                                        .py_px()
                                                        .children([
                                                            div().px_1().py_px().rounded_sm()
                                                                .bg(rgb(method_clr))
                                                                .text_xs().text_color(rgb(0xffffff))
                                                                .child(tab_method),
                                                            div()
                                                                .text_color(if is_active { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                                                                .max_w(px(90.0))
                                                                .overflow_hidden()
                                                                .text_ellipsis()
                                                                .child(tab_name),
                                                        ]),
                                                ]
                                            })
                                    }))
                                    // 新增标签按钮
                                    .child(
                                        div()
                                            .id("new-tab-btn")
                                            .h(px(40.0))
                                            .w(px(36.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .text_color(rgb(0x888888))
                                            .hover(|s| s.bg(rgb(0x333333)).text_color(rgb(0xffffff)))
                                            .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                this.add_tab(window, cx);
                                            }))
                                            .child(Icon::new(IconName::Plus).small())
                                    ),
                                // 请求构造器
                                div()
                                    .h(px(self.request_builder_height))
                                    .flex()
                                    .w_full()
                                    .flex_col()
                                    .overflow_hidden()
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
                                                    .flex()
                                                    .h(px(34.0))
                                                    .w(px(95.0))
                                                    .mr_2()
                                                    .child(
                                                        Button::new("send")
                                                        .px_4()
                                                        .rounded_sm()
                                                        .bg(if is_loading { rgb(0x666666) } else { rgb(0x3b82f6) })
                                                        .text_color(rgb(0xffffff))
                                                        .font_semibold()
                                                        .icon(if is_loading {
                                                            IconName::LoaderCircle
                                                        } else {
                                                            IconName::Play
                                                        })
                                                        .label(if is_loading { "Sending" } else { "Send" })
                                                        .flex_none()
                                                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                            let url = this.url_input.read(cx).value().to_string();
                                                            let method = this.method_select.read(cx).selected_value()
                                                                .unwrap_or(&gpui::SharedString::from("GET")).clone();
                                                            this.url = url;
                                                            this.method = method.to_string();
                                                            this.send_request(_window, cx);
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
                                                .overflow_y_hidden()
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
                                                        .mt_1()
                                                        .children(params.iter().enumerate().map(|(idx, param)| {
                                                            div()
                                                                .mt_1()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_2()
                                                                .items_center()
                                                                .children([
                                                                    div()
                                                                        .w(px(24.0))
                                                                        .h(px(24.0))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .text_sm()
                                                                        .text_color(if param.enabled { rgb(0x22c55e) } else { rgb(0x666666) })
                                                                        .child(if param.enabled { "✓" } else { "○" }),
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
                                                                    div()
                                                                        .w(px(20.0))
                                                                        .h(px(20.0))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .child(
                                                                            Button::new(idx.to_string())
                                                                                .small()
                                                                                .icon(IconName::Close)
                                                                                .text_color(rgb(0x888888))
                                                                                .bg(rgb(0x1e1e1e))
                                                                                .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                    this.remove_param(idx, cx);
                                                                                }))
                                                                        ),
                                                                ])
                                                        })),
                                                    // 添加行按钮
                                                    div()
                                                        .mt_2()
                                                        .px_1()
                                                        .py_1()
                                                        .child(
                                                            Button::new("add-param")
                                                                .px_2()
                                                                .py_1()
                                                                .text_sm()
                                                                .icon(IconName::Plus)
                                                                .text_color(rgb(0x3b82f6))
                                                                .bg(rgb(0x1e1e1e))
                                                                .label("Add parameter")
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
                                                .overflow_y_hidden()
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
                                                        .mt_1()
                                                        .children(headers.iter().enumerate().map(|(idx, header)| {
                                                            div()
                                                                .mt_1()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_2()
                                                                .items_center()
                                                                .children([
                                                                    div()
                                                                        .w(px(24.0))
                                                                        .h(px(24.0))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .text_sm()
                                                                        .text_color(if header.enabled { rgb(0x22c55e) } else { rgb(0x666666) })
                                                                        .child(if header.enabled { "✓" } else { "○" }),
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
                                                                    div()
                                                                        .w(px(20.0))
                                                                        .h(px(20.0))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .child(
                                                                            Button::new(idx.to_string())
                                                                                .small()
                                                                                .icon(IconName::Close)
                                                                                .text_color(rgb(0x888888))
                                                                                .bg(rgb(0x1e1e1e))
                                                                                .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                    this.remove_header(idx, cx);
                                                                                }))
                                                                        ),
                                                                ])
                                                        })),
                                                    // 添加行按钮
                                                    div()
                                                        .mt_2()
                                                        .px_1()
                                                        .py_1()
                                                        .child(
                                                            Button::new("add-header")
                                                                .px_2()
                                                                .py_1()
                                                                .text_sm()
                                                                .icon(IconName::Plus)
                                                                .text_color(rgb(0x3b82f6))
                                                                .bg(rgb(0x1e1e1e))
                                                                .label("Add Header")
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
                                                .overflow_y_hidden()
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
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if body_state.body_type == BodyType::None { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if body_state.body_type == BodyType::None { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_body_type(BodyType::None.to_index(), cx);
                                                                        }))
                                                                        .child("none"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if body_state.body_type == BodyType::FormData { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if body_state.body_type == BodyType::FormData { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_body_type(BodyType::FormData.to_index(), cx);
                                                                        }))
                                                                        .child("form-data"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if body_state.body_type == BodyType::UrlEncoded { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if body_state.body_type == BodyType::UrlEncoded { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_body_type(BodyType::UrlEncoded.to_index(), cx);
                                                                        }))
                                                                        .child("x-www-form-urlencoded"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if body_state.body_type == BodyType::Raw { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if body_state.body_type == BodyType::Raw { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_body_type(BodyType::Raw.to_index(), cx);
                                                                        }))
                                                                        .child("raw"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if body_state.body_type == BodyType::Binary { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if body_state.body_type == BodyType::Binary { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_body_type(BodyType::Binary.to_index(), cx);
                                                                        }))
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
                                                                                    .cursor_pointer()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .rounded_sm()
                                                                                    .bg(if body_state.raw_format == RawFormat::Json { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                    .text_color(if body_state.raw_format == RawFormat::Json { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        this.set_raw_format(RawFormat::Json.to_index(), cx);
                                                                                    }))
                                                                                    .child("JSON"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .cursor_pointer()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .rounded_sm()
                                                                                    .bg(if body_state.raw_format == RawFormat::Xml { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                    .text_color(if body_state.raw_format == RawFormat::Xml { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        this.set_raw_format(RawFormat::Xml.to_index(), cx);
                                                                                    }))
                                                                                    .child("XML"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .cursor_pointer()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .rounded_sm()
                                                                                    .bg(if body_state.raw_format == RawFormat::Text { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                    .text_color(if body_state.raw_format == RawFormat::Text { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        this.set_raw_format(RawFormat::Text.to_index(), cx);
                                                                                    }))
                                                                                    .child("Text"),
                                                                                div()
                                                                                    .text_sm()
                                                                                    .cursor_pointer()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .rounded_sm()
                                                                                    .bg(if body_state.raw_format == RawFormat::Html { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                    .text_color(if body_state.raw_format == RawFormat::Html { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        this.set_raw_format(RawFormat::Html.to_index(), cx);
                                                                                    }))
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
                                                    } else if body_state.body_type == BodyType::FormData {
                                                        div()
                                                            .flex_col()
                                                            .flex_1()
                                                            .gap_2()
                                                            .children([
                                                                // 表头 - 参考 params 样式, Type 在 Key 和 Value 中间
                                                                div()
                                                                    .flex()
                                                                    .flex_row()
                                                                    .gap_2()
                                                                    .mb_1()
                                                                    .children([
                                                                        div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                                        div().flex_1().text_xs().text_color(rgb(0x888888)).child("Key"),
                                                                        div().w(px(90.0)).text_xs().text_color(rgb(0x888888)).child("Type"),
                                                                        div().flex_1().text_xs().text_color(rgb(0x888888)).child("Value"),
                                                                        div().w(px(30.0)).text_xs().text_color(rgb(0x888888)).child(""),
                                                                    ]),
                                                                // Form-data 条目
                                                                div()
                                                                    .flex_col()
                                                                    .gap_2()
                                                                    .py_1()
                                                                    .mt_1()
                                                                    .children(body_state.form_data.iter().enumerate().map(|(idx, entry)| {
                                                                        let is_file = entry.param_type == crate::ui::FormDataParamType::File;
                                                                        let value_entity = match &entry.value {
                                                                            crate::ui::FormDataValue::Text(e) => Some(e.clone()),
                                                                            crate::ui::FormDataValue::File(_, _) => None,
                                                                        };
                                                                        let param_type_copy = entry.param_type;
                                                                        div()
                                                                            .mt_1()
                                                                            .flex()
                                                                            .flex_row()
                                                                            .gap_2()
                                                                            .items_center()
                                                                            .children([
                                                                                // 启用/禁用复选框
                                                                                div()
                                                                                    .w(px(24.0))
                                                                                    .h(px(24.0))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .justify_center()
                                                                                    .text_sm()
                                                                                    .text_color(if entry.enabled { rgb(0x22c55e) } else { rgb(0x666666) })
                                                                                    .cursor_pointer()
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        this.toggle_form_data_entry(idx, cx);
                                                                                    }))
                                                                                    .child(if entry.enabled { "✓" } else { "○" }),
                                                                                // Key 输入框
                                                                                div().flex_1()
                                                                                    .child(
                                                                                        Input::new(&entry.key)
                                                                                            .small()
                                                                                            .h(px(32.0))
                                                                                            .bg(rgb(0x2d2d2d))
                                                                                            .border_1()
                                                                                            .border_color(rgb(0x444444))
                                                                                            .text_color(rgb(0xe0e0e0)),
                                                                                    ),
                                                                                // 类型选择 - 循环切换
                                                                                div()
                                                                                    .w(px(90.0))
                                                                                    .h(px(28.0))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .justify_center()
                                                                                    .cursor_pointer()
                                                                                    .rounded_sm()
                                                                                    .bg(rgb(0x2d2d2d))
                                                                                    .border_1()
                                                                                    .border_color(rgb(0x444444))
                                                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                        // 循环切换类型: Text -> Boolean -> Number -> File -> Array -> Text
                                                                                        let next_type = match param_type_copy {
                                                                                            crate::ui::FormDataParamType::Text => crate::ui::FormDataParamType::Boolean,
                                                                                            crate::ui::FormDataParamType::Boolean => crate::ui::FormDataParamType::Number,
                                                                                            crate::ui::FormDataParamType::Number => crate::ui::FormDataParamType::File,
                                                                                            crate::ui::FormDataParamType::File => crate::ui::FormDataParamType::Array,
                                                                                            crate::ui::FormDataParamType::Array => crate::ui::FormDataParamType::Text,
                                                                                        };
                                                                                        this.set_form_data_param_type(idx, next_type, _window, cx);
                                                                                    }))
                                                                                    .children([
                                                                                        div().text_sm().text_color(rgb(0xe0e0e0)).child(match entry.param_type {
                                                                                            crate::ui::FormDataParamType::Text => "Text",
                                                                                            crate::ui::FormDataParamType::Boolean => "Boolean",
                                                                                            crate::ui::FormDataParamType::Number => "Number",
                                                                                            crate::ui::FormDataParamType::File => "File",
                                                                                            crate::ui::FormDataParamType::Array => "Array",
                                                                                        }),
                                                                                        div().h(px(24.0)).text_sm().text_color(rgb(0x888888)).child("▼"),
                                                                                    ]),
                                                                                // Value 输入框 - 非 File 类型时显示
                                                                                if !is_file {
                                                                                    div().flex_1()
                                                                                        .child(
                                                                                            Input::new(&entry.value.get_input_entity())
                                                                                                .small()
                                                                                                .h(px(32.0))
                                                                                                .bg(rgb(0x2d2d2d))
                                                                                                .border_1()
                                                                                                .border_color(rgb(0x444444))
                                                                                                .text_color(rgb(0xe0e0e0)),
                                                                                        )
                                                                                } else {
                                                                                    // File类型 - 显示文件路径或占位文本
                                                                                    let file_path = entry.value.get_input_entity().read(cx).value().to_string();
                                                                                    let display_path = file_path.clone();
                                                                                    let is_placeholder = display_path.is_empty();
                                                                                    div()
                                                                                        .flex_1()
                                                                                        .h(px(28.0))
                                                                                       
                                                                        
                                                                                        .items_center()
                                                                                        .rounded_sm()
                                                                                        .bg(rgb(0x2d2d2d))
                                                                                        .border_1()
                                                                                        .border_color(rgb(0x444444))
                                                                                        .cursor_pointer()
                                                                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                            this.pick_file_for_form_data(idx, _window, cx);
                                                                                        }))
                                                                                        .child(
                                                                                            div()
                                                                                            .flex()
                                                                                                .h(px(24.0))
                                                                                                .flex_1()
                                                                                                .text_sm()
                                                                                                .text_color(if is_placeholder { rgb(0x666666) } else { rgb(0xe0e0e0) })
                                                                                                .overflow_hidden()
                                                                                                .text_ellipsis()
                                                                                                .child(if is_placeholder { "Select file...".to_string() } else { display_path }),
                                                                                        )
                                                                                },
                                                                                // 删除按钮
                                                                                div()
                                                                                    .w(px(24.0))
                                                                                    .h(px(24.0))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .justify_center()
                                                                                    .child(
                                                                                        Button::new(idx.to_string())
                                                                                            .small()
                                                                                            .icon(IconName::Close)
                                                                                            .text_color(rgb(0x888888))
                                                                                            .bg(rgb(0x1e1e1e))
                                                                                            .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                                this.remove_form_data_entry(idx);
                                                                                                cx.notify();
                                                                                            }))
                                                                                    ),
                                                                            ])
                                                                    })),
                                                                // 添加行按钮
                                                                div()
                                                                    .mt_2()
                                                                    .px_1()
                                                                    .py_1()
                                                                    .child(
                                                                        Button::new("add-formdata")
                                                                            .px_2()
                                                                            .py_1()
                                                                            .text_sm()
                                                                            .icon(IconName::Plus)
                                                                            .text_color(rgb(0x3b82f6))
                                                                            .bg(rgb(0x1e1e1e))
                                                                            .label("Add form data")
                                                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                this.add_form_data_entry(_window, cx);
                                                                            }))
                                                                    ),
                                                            ])
                                                    } else if body_state.body_type == BodyType::UrlEncoded {
                                                        div()
                                                            .flex_col()
                                                            .flex_1()
                                                            .gap_2()
                                                            .children([
                                                                // 表头 - 参考 params 样式
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
                                                                // URL-encoded 条目
                                                                div()
                                                                    .flex_col()
                                                                    .gap_2()
                                                                    .py_1()
                                                                    .mt_1()
                                                                    .children(body_state.urlencoded_data.iter().enumerate().map(|(idx, entry)| {
                                                                        div()
                                                                            .mt_1()
                                                                            .flex()
                                                                            .flex_row()
                                                                            .gap_2()
                                                                            .items_center()
                                                                            .children([
                                                                                div()
                                                                                    .w(px(24.0))
                                                                                    .h(px(24.0))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .justify_center()
                                                                                    .text_sm()
                                                                                    .text_color(if entry.enabled { rgb(0x22c55e) } else { rgb(0x666666) })
                                                                                    .child(if entry.enabled { "✓" } else { "○" }),
                                                                                div().flex_1()
                                                                                    .child(
                                                                                        Input::new(&entry.key)
                                                                                            .small()
                                                                                            .h(px(32.0))
                                                                                            .bg(rgb(0x2d2d2d))
                                                                                            .border_1()
                                                                                            .border_color(rgb(0x444444))
                                                                                            .text_color(rgb(0xe0e0e0)),
                                                                                    ),
                                                                                div().flex_1()
                                                                                    .child(
                                                                                        Input::new(&entry.value.get_input_entity())
                                                                                            .small()
                                                                                            .h(px(32.0))
                                                                                            .bg(rgb(0x2d2d2d))
                                                                                            .border_1()
                                                                                            .border_color(rgb(0x444444))
                                                                                            .text_color(rgb(0xe0e0e0)),
                                                                                    ),
                                                                                div()
                                                                                    .w(px(24.0))
                                                                                    .h(px(24.0))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .justify_center()
                                                                                    .child(
                                                                                        Button::new(idx.to_string())
                                                                                            .small()
                                                                                            .icon(IconName::Close)
                                                                                            .text_color(rgb(0x888888))
                                                                                            .bg(rgb(0x1e1e1e))
                                                                                            .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                                this.remove_urlencoded_entry(idx);
                                                                                                cx.notify();
                                                                                            }))
                                                                                    ),
                                                                            ])
                                                                    })),
                                                                // 添加行按钮
                                                                div()
                                                                    .mt_2()
                                                                    .px_1()
                                                                    .py_1()
                                                                    .child(
                                                                        Button::new("add-urlencoded")
                                                                            .px_2()
                                                                            .py_1()
                                                                            .text_sm()
                                                                            .icon(IconName::Plus)
                                                                            .text_color(rgb(0x3b82f6))
                                                                            .bg(rgb(0x1e1e1e))
                                                                            .label("Add URL-encoded")
                                                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                this.add_urlencoded_entry(_window, cx);
                                                                            }))
                                                                    ),
                                                            ])
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
                                                .overflow_y_hidden()
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
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if auth_type == AuthType::NoAuth { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if auth_type == AuthType::NoAuth { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_auth_type(AuthType::NoAuth.to_index(), _window, cx);
                                                                        }))
                                                                        .child("No Auth"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if auth_type == AuthType::BearerToken { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if auth_type == AuthType::BearerToken { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_auth_type(AuthType::BearerToken.to_index(), _window, cx);
                                                                        }))
                                                                        .child("Bearer Token"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if auth_type == AuthType::BasicAuth { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if auth_type == AuthType::BasicAuth { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_auth_type(AuthType::BasicAuth.to_index(), _window, cx);
                                                                        }))
                                                                        .child("Basic Auth"),
                                                                    div()
                                                                        .text_sm()
                                                                        .cursor_pointer()
                                                                        .px_2()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .bg(if auth_type == AuthType::ApiKey { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                        .text_color(if auth_type == AuthType::ApiKey { rgb(0xffffff) } else { rgb(0x888888) })
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.set_auth_type(AuthType::ApiKey.to_index(), _window, cx);
                                                                        }))
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
                                                                                .cursor_pointer()
                                                                                .px_2()
                                                                                .py_1()
                                                                                .rounded_sm()
                                                                                .bg(if auth.location_value == ApiKeyLocation::Header { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                .text_color(if auth.location_value == ApiKeyLocation::Header { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                    this.toggle_api_key_location(cx);
                                                                                }))
                                                                                .child("Header"),
                                                                            div()
                                                                                .text_sm()
                                                                                .cursor_pointer()
                                                                                .px_2()
                                                                                .py_1()
                                                                                .rounded_sm()
                                                                                .bg(if auth.location_value == ApiKeyLocation::Query { rgb(0x3b3b3b) } else { rgb(0x2d2d2d) })
                                                                                .text_color(if auth.location_value == ApiKeyLocation::Query { rgb(0xffffff) } else { rgb(0x888888) })
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                    this.toggle_api_key_location(cx);
                                                                                }))
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
                                                .overflow_y_hidden()
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
                                                .overflow_y_hidden()
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
                                                .overflow_y_hidden()
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
                                                                .text_color(if settings.follow_redirects { rgb(0x22c55e) } else { rgb(0x888888) })
                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    this.toggle_follow_redirects(cx);
                                                                }))
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
                                                                .text_color(if settings.verify_ssl { rgb(0x22c55e) } else { rgb(0x888888) })
                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    this.toggle_verify_ssl(cx);
                                                                }))
                                                                .child(if settings.verify_ssl { "ON" } else { "OFF" }),
                                                        ]),
                                                ])
                                        } else {
                                            div().flex_1().hidden()
                                        },
                                    ]),
                                // Splitter（可拖拽调整上下区域大小）
                                div()
                                    .h(px(12.0))
                                    .w_full()
                                    .bg(rgb(0x333333))
                                    .cursor_row_resize()
                                    .hover(|s| s.bg(rgb(0x3b82f6)))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        let y: f32 = event.position.y.into();
                                        this.start_splitter_drag(y);
                                        cx.notify();
                                    }))
                                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        if this.splitter_dragging {
                                            let y: f32 = event.position.y.into();
                                            this.update_splitter_drag(y);
                                            cx.notify();
                                        }
                                    }))
                                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        this.end_splitter_drag();
                                        cx.notify();
                                    })),
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
                                                    let theme = Theme::from_str(&self.app_state.theme_name);
                                                    let header_row = div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .justify_between()
                                                        .w_full()
                                                        .mb_2()
                                                        .children([
                                                            // 状态徽章
                                                            div()
                                                                .px_2()
                                                                .py_px()
                                                                .rounded_sm()
                                                                .bg(rgb(if (200..300).contains(&resp.status) { 0x22c55e } else { 0xef4444 }))
                                                                .text_color(rgb(0xffffff))
                                                                .child(format!("{} {}", resp.status, resp.status_text())),
                                                            // 模式选择按钮
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_2()
                                                                .children([
                                                                    Button::new("pretty")
                                                                        .label("Pretty")
                                                                        .px_3()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .text_sm()
                                                                        .bg(if self.body_view_mode == BodyViewMode::Pretty { theme.accent } else { theme.input_background })
                                                                        .text_color(if self.body_view_mode == BodyViewMode::Pretty { theme.accent_foreground } else { theme.muted_foreground })
                                                                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.body_view_mode = BodyViewMode::Pretty;
                                                                            cx.notify();
                                                                        })),
                                                                    Button::new("raw")
                                                                        .label("Raw")
                                                                        .px_3()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .text_sm()
                                                                        .bg(if self.body_view_mode == BodyViewMode::Raw { theme.accent } else { theme.input_background })
                                                                        .text_color(if self.body_view_mode == BodyViewMode::Raw { theme.accent_foreground } else { theme.muted_foreground })
                                                                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.body_view_mode = BodyViewMode::Raw;
                                                                            cx.notify();
                                                                        })),
                                                                    Button::new("preview")
                                                                        .label("Preview")
                                                                        .px_3()
                                                                        .py_1()
                                                                        .rounded_sm()
                                                                        .text_sm()
                                                                        .bg(if self.body_view_mode == BodyViewMode::Preview { theme.accent } else { theme.input_background })
                                                                        .text_color(if self.body_view_mode == BodyViewMode::Preview { theme.accent_foreground } else { theme.muted_foreground })
                                                                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.body_view_mode = BodyViewMode::Preview;
                                                                            cx.notify();
                                                                        })),
                                                                ]),
                                                            // Time 和 Size 在右边
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_4()
                                                                .children([
                                                                    div().text_color(rgb(0x888888)).child(format!("Time: {}ms", resp.time_ms)),
                                                                    div().text_color(rgb(0x888888)).child(format!("Size: {} bytes", resp.size_bytes)),
                                                                ]),
                                                        ]);

                                                    // 根据视图模式显示内容
                                                    let content: Div = match self.body_view_mode {
                                                        BodyViewMode::Pretty => {
                                                            div()
                                                                .flex_1()
                                                                .flex_col()
                                                                .overflow_hidden()
                                                                .bg(rgb(0x2d2d2d))
                                                                .border_1()
                                                                .border_color(rgb(0x444444))
                                                                .rounded_md()
                                                                .child(
                                                                    Input::new(&self.response_input)
                                                                        .w_full()
                                                                        .flex_1(),
                                                                )
                                                        },
                                                        BodyViewMode::Raw => {
                                                            div()
                                                                .flex_1()
                                                                .flex_col()
                                                                .overflow_hidden()
                                                                .bg(rgb(0x2d2d2d))
                                                                .border_1()
                                                                .border_color(rgb(0x444444))
                                                                .rounded_md()
                                                                .child(
                                                                    Input::new(&self.response_input)
                                                                        .w_full()
                                                                        .flex_1(),
                                                                )
                                                        },
                                                        BodyViewMode::Preview => {
                                                            div()
                                                                .flex_1()
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .text_color(rgb(0x666666))
                                                                .child("Preview mode not implemented")
                                                        },
                                                    };

                                                    div()
                                                        .flex_col()
                                                        .gap_2()
                                                        .flex_1()
                                                        .overflow_hidden()
                                                        .children([
                                                            header_row,
                                                            content,
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
