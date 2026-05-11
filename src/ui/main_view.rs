//! 主视图模块
//!
//! 应用主界面，包含侧边栏和请求/响应面板

use crate::app::database::{Environment, Folder, HistoryEntry, SavedRequest};
use crate::app::HttpResponse;
use crate::http::HttpRequest;
use crate::app::history::CreateHistoryEntry;
use crate::ui::{
    count_lines, json_editor, ApiKeyLocation, AuthState, AuthType, BodyState, BodyType,
    HeaderEntry, RawFormat, RequestSettings, ScriptState,
    SettingsInputs, Theme,
};
use crate::ui::dialogs::{EnvDialogState, FolderDialogState, MoveDialogState, render_env_dialog_overlay, render_folder_dialog_overlay, render_move_dialog_overlay};
use crate::ui::sidebar::CollectionItem;
use crate::ui::sidebar::{build_collection_tree, render_collection_panel};
use crate::ui::components::{tooltip_popup, popup_panel};
use crate::ui::sidebar::{render_folder_context_menu, render_request_context_menu};
use gpui::prelude::*;
use gpui::*;
use gpui::InteractiveElement;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::button::Button;
use gpui_component::scroll::ScrollableElement;
use gpui_component::scroll::Scrollable;
use gpui_component::{Disableable, Icon, IconName, IndexPath, Sizable, StyledExt, WindowExt};
use gpui_component::dialog::{Dialog, DialogHeader, DialogTitle};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
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

/// 格式化字节大小为可读字符串
fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
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
#[derive(Debug)]
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
#[derive(Clone, Copy, PartialEq, Debug)]
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
    pub(crate) app_state: Arc<std::sync::Mutex<crate::app::AppState>>,
    pub method: String,
    pub url: String,
    pub(crate) request_tabs: Vec<RequestTab>,
    pub(crate) active_tab: usize,
    pub(crate) response: Option<HttpResponse>,
    pub(crate) response_tab: ResponseTab,
    pub(crate) body_view_mode: BodyViewMode,
    pub(crate) is_loading: bool,
    pub(crate) error_message: Option<String>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) sidebar_tab: SidebarTab,
    pub(crate) show_settings_popover: bool,
    pub(crate) context_menu_target: Option<String>,
    pub(crate) hovered_item_name: Option<String>,
    pub(crate) hovered_item_y: Option<f32>,
    pub(crate) hovered_item_x: Option<f32>,
    pub(crate) context_menu_pos: Option<(f32, f32)>,
    pub(crate) env_menu_target: Option<String>,
    pub(crate) env_menu_pos: Option<(f32, f32)>,
    pub(crate) history: Vec<HistoryEntry>,
    pub(crate) saved_requests: Vec<crate::app::database::SavedRequest>,
    pub(crate) folders: Vec<crate::app::database::Folder>,
    pub(crate) environments: Vec<crate::app::database::Environment>,
    pub(crate) active_environment_name: Option<String>,
    pub(crate) env_dialog_state: Arc<Mutex<EnvDialogState>>,
    pub(crate) folder_dialog_state: Arc<Mutex<FolderDialogState>>,
    pub(crate) move_dialog_state: Arc<Mutex<MoveDialogState>>,
    pub(crate) expanded_folders: HashSet<String>,
    pub(crate) collection_items: Vec<CollectionItem>,
    pub(crate) needs_collections_refresh: bool,
    pub(crate) needs_drop_refresh: Arc<AtomicBool>,
    pub(crate) url_input: Entity<InputState>,
    pub(crate) _url_change_sub: gpui::Subscription,
    pub(crate) _param_input_subs: Vec<gpui::Subscription>,
    pub(crate) _header_subs: Vec<gpui::Subscription>,
    pub(crate) method_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) builder_tab: BuilderTab,
    pub(crate) params: Vec<ParamEntry>,
    pub(crate) headers: Vec<HeaderEntry>,
    pub(crate) body_state: BodyState,
    pub(crate) body_type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) raw_format_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) auth_type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) auth_state: AuthState,
    pub(crate) script_state: ScriptState,
    pub(crate) settings: RequestSettings,
    pub(crate) settings_inputs: SettingsInputs,
    pub(crate) is_importing_curl: bool,
    pub(crate) last_synced_url: String,
    pub(crate) response_input: Entity<InputState>,
    pub(crate) response_xml_input: Entity<InputState>,
    pub(crate) response_text_input: Entity<InputState>,
    pub(crate) response_html_input: Entity<InputState>,
    pub(crate) response_header_inputs: Vec<(Entity<InputState>, Entity<InputState>)>,
    pub(crate) response_raw_format: RawFormat,
    pub(crate) response_raw_format_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) response_soft_wrap: bool,
    pub(crate) next_tab_id: usize,
    pub(crate) splitter_dragging: bool,
    pub(crate) splitter_start_y: f32,
    pub(crate) request_builder_height: f32,
    pub(crate) response_editor_height: f32,
    pub(crate) response_editor_dragging: bool,
    pub(crate) response_editor_start_y: f32,
    pub(crate) save_request_dialog: Arc<Mutex<SaveRequestDialog>>,
}

/// 保存到收藏夹的对话框状态
pub(crate) struct SaveRequestDialog {
    pub(crate) visible: bool,
    pub(crate) name_input: Entity<InputState>,
    pub(crate) pending_request: Option<crate::app::database::SavedRequest>,
    pub(crate) needs_refresh: bool,
}

impl SaveRequestDialog {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<MainView>) -> Self {
        Self {
            visible: false,
            name_input: cx.new(|cx| InputState::new(window, cx).default_value("")),
            pending_request: None,
            needs_refresh: false,
        }
    }

    pub(crate) fn open(
        &mut self,
        default_name: String,
        request: crate::app::database::SavedRequest,
        window: &mut Window,
        cx: &mut Context<MainView>,
    ) {
        self.name_input.update(cx, |s, cx| {
            s.set_value(&default_name, window, cx);
        });
        self.pending_request = Some(request);
        self.needs_refresh = false;
        self.visible = true;
    }
}

impl MainView {
    /// 获取翻译文本
    pub(crate) fn t(&self, key: &str) -> String {
        self.app_state.lock().unwrap().i18n.get(key)
    }

    /// 创建构建器标签页按钮（带i18n支持）
    fn builder_tab_button(&self, cx: &Context<Self>, label_key: &str, tab: BuilderTab, current_tab: BuilderTab, id: impl Into<ElementId>) -> impl IntoElement {
        let theme = Theme::from_str(&self.app_state.lock().unwrap().theme_name);
        let is_active = current_tab == tab;
        div()
            .id(id)
            .min_w(px(80.0))
            .px_4()
            .py_2()
            .text_sm()
            .cursor_pointer()
            .text_color(if is_active { theme.accent_foreground } else { theme.muted_foreground })
            .bg(if is_active { theme.code_background } else { theme.background })
            .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_builder_tab(tab, cx);
            }))
            .child(self.t(label_key))
    }

    /// 创建响应标签页按钮（带i18n支持）
    fn response_tab_button(&self, cx: &Context<Self>, label_key: &str, tab: ResponseTab, current_tab: ResponseTab, id: impl Into<ElementId>) -> impl IntoElement {
        let theme = Theme::from_str(&self.app_state.lock().unwrap().theme_name);
        let is_active = current_tab == tab;
        div()
            .id(id)
            .min_w(px(70.0))
            .px_3()
            .py_2()
            .text_sm()
            .cursor_pointer()
            .text_color(if is_active { theme.accent_foreground } else { theme.muted_foreground })
            .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_response_tab(tab, cx);
            }))
            .child(self.t(label_key))
    }

    /// 创建新的主视图
    pub fn new(app_state: Arc<std::sync::Mutex<crate::app::AppState>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 加载历史记录
        let history = app_state.lock().unwrap().db
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
                    if url == this.last_synced_url {
                        return;
                    }
                    // 检测cURL命令
                    if url.trim().starts_with("curl ") || url.trim().starts_with("curl\n") {
                        if let Err(e) = this.import_curl(&url, _window, cx) {
                            log::warn!("解析cURL命令失败: {}", e);
                        }
                    } else if url.contains('?') {
                        this.parse_url_to_params_internal(&url, _window, cx);
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

        // 创建 Body 状态（JSON 编辑器使用 code_editor 模式）
        let raw_content = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(r#"{"key": "value"}"#)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                        });
        // XML 格式的 raw_content - 复制JSON编辑框
        let raw_content_xml = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(r#"<root></root>"#)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                        });
        // Text 格式的 raw_content - 复制JSON编辑框
        let raw_content_text = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(r#"plain text"#)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                        });
        // HTML 格式的 raw_content - 复制JSON编辑框
        let raw_content_html = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(r#"<html></html>"#)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                        });
        let body_state = BodyState::new(raw_content, raw_content_xml, raw_content_text, raw_content_html);

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
                .multi_line(true)
                .soft_wrap(true)
                .line_number(true)
                                .default_value("")
        });

        // 创建响应体 XML 输入状态（使用 code_editor）
        let response_xml_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                                .default_value("")
        });

        // 创建响应体 Text 输入状态（使用 code_editor）
        let response_text_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                                .default_value("")
        });

        // 创建响应体 Html 输入状态（使用 code_editor）
        let response_html_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                                .default_value("")
        });

        // 响应头输入状态（动态创建，每个头一个键值对）
        let response_header_inputs: Vec<(Entity<InputState>, Entity<InputState>)> = Vec::new();

        // 创建响应体Raw格式选择器
        let response_raw_format_select = BodyState::create_raw_format_select(window, cx);

        // 获取默认标签页名称
        let default_tab_name = app_state.lock().unwrap().i18n.get("sidebar.new_request");
        let saved_requests = app_state.lock().unwrap().db.get_saved_requests().unwrap_or_default();
        let folders = app_state.lock().unwrap().db.get_folders().unwrap_or_default();
        let environments = app_state.lock().unwrap().db.get_environments().unwrap_or_default();
        let active_environment_name = environments.iter()
            .find(|e| e.is_active)
            .map(|e| e.name.clone());
        let env_dialog_state = Arc::new(Mutex::new(EnvDialogState::new(window, cx)));
        let folder_dialog_state = Arc::new(Mutex::new(FolderDialogState::new(window, cx)));
        let move_dialog_state = Arc::new(Mutex::new(MoveDialogState::new()));
        let save_request_dialog = Arc::new(Mutex::new(SaveRequestDialog::new(window, cx)));

        Self {
            app_state,
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            request_tabs: vec![RequestTab {
                id: 1,
                method: "GET".to_string(),
                url: "https://httpbin.org/get".to_string(),
                name: default_tab_name,
            }],
            active_tab: 0,
            response: None,
            response_tab: ResponseTab::Body,
            body_view_mode: BodyViewMode::Pretty,
            is_loading: false,
            error_message: None,
            sidebar_collapsed: false,
            sidebar_tab: SidebarTab::Collections,
            show_settings_popover: false,
            context_menu_target: None,
            hovered_item_name: None,
            hovered_item_y: None,
            hovered_item_x: None,
            context_menu_pos: None,
            env_menu_target: None,
            env_menu_pos: None,
            expanded_folders: HashSet::new(),
            collection_items: crate::ui::sidebar::build_collection_tree(
                &folders,
                &saved_requests,
                &HashSet::new(),
            ),
            needs_collections_refresh: false,
            needs_drop_refresh: Arc::new(AtomicBool::new(false)),
            history,
            saved_requests,
            folders,
            environments,
            active_environment_name,
            env_dialog_state,
            folder_dialog_state,
            move_dialog_state,
            url_input,
            _url_change_sub,
            _param_input_subs: Vec::new(),
            _header_subs: Vec::new(),
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
            last_synced_url: String::new(),
            response_input,
            response_xml_input,
            response_text_input,
            response_html_input,
            response_header_inputs,
            response_raw_format: RawFormat::Json,
            response_raw_format_select,
            response_soft_wrap: false,
            next_tab_id: 2,
            splitter_dragging: false,
            splitter_start_y: 0.0,
            request_builder_height: 300.0,
            response_editor_height: 400.0,
            response_editor_dragging: false,
            response_editor_start_y: 0.0,
            save_request_dialog,
        }
    }

    /// 发送HTTP请求
    pub fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }

        self.is_loading = true;
        self.error_message = None;

        // 0. 同步 params 到 URL（确保最新修改反映到 URL bar）
        self.sync_params_to_url(window, cx);

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

        // 6. 构建完整URL（先替换环境变量再检查scheme，避免变量值含http导致重复添加）
        let resolved_url = self.app_state.lock().unwrap().env_manager.replace_variables(&self.url);
        let url_with_scheme = if !resolved_url.starts_with("http://") && !resolved_url.starts_with("https://") {
            format!("http://{}", resolved_url)
        } else {
            resolved_url
        };

        let base_url = if let Some(query_start) = url_with_scheme.find('?') {
            url_with_scheme[..query_start].to_string()
        } else {
            url_with_scheme
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

        // 7. API Key query 参数
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

        let headers_text_for_history: String = all_headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let body_for_history = body.clone();
        let method = self.method.clone();
        let url = self.url.clone();
        let app_state = self.app_state.clone();

        let request = HttpRequest {
            method: method.clone(),
            url: full_url,
            headers: all_headers,
            body,
            content_type,
            text_fields,
            file_fields,
        };

        // 使用 cx.spawn_in 异步发送请求，不阻塞 UI 线程
        cx.spawn_in(window, async move |this: WeakEntity<MainView>, cx| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(app_state.lock().unwrap().send_request(request));

            this.update_in(cx, |this, window, cx| {
                    match result {
                        Ok(response) => {
                            let history_entry = CreateHistoryEntry {
                                method: method.clone(),
                                url: url.clone(),
                                headers: Some(headers_text_for_history),
                                body: body_for_history,
                                response_status: Some(response.status as i32),
                                response_headers: Some(serde_json::to_string(&response.headers).unwrap_or_default()),
                                response_body: Some(response.body.clone()),
                                response_time_ms: Some(response.time_ms),
                                response_size: Some(response.size_bytes),
                            };

                            if let Err(e) = this.app_state.lock().unwrap().db.add_history(&history_entry.into_history_entry()) {
                                log::error!("保存历史记录失败: {}", e);
                            }

                            this.response = Some(response.clone());
                            let content_type = response.detect_content_type();
                            this.response_raw_format = RawFormat::detect(content_type.as_deref(), &response.body);
                            let json_body = RawFormat::Json.format_body(&response.body);
                            this.response_input.update(cx, |state, cx| {
                                state.set_value(&json_body, window, cx);
                            });
                            this.response_xml_input.update(cx, |state, cx| {
                                state.set_value(&response.body, window, cx);
                            });
                            this.response_text_input.update(cx, |state, cx| {
                                state.set_value(&response.body, window, cx);
                            });
                            this.response_html_input.update(cx, |state, cx| {
                                state.set_value(&response.body, window, cx);
                            });
                            this.rebuild_response_header_inputs(&response.headers, window, cx);
                            if let Ok(hist) = this.app_state.lock().unwrap().db.get_history(50, 0) {
                                this.history = hist;
                            }
                        }
                        Err(e) => {
                            this.error_message = Some(e);
                        }
                    }

                    this.is_loading = false;
                    cx.notify();
            }).ok();
        }).detach();
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
        log::info!("set_sidebar_tab: 切换到标签页 {:?}", tab);
        self.sidebar_tab = tab;
        if tab == SidebarTab::Collections {
            self.needs_collections_refresh = true;
        }
        cx.notify();
    }

    /// 切换侧边栏折叠状态
    #[allow(dead_code)]
    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    /// 计算 body 内容的行数
    fn calculate_body_line_count(body_state: &BodyState, cx: &Context<Self>) -> usize {
        if body_state.body_type == BodyType::Raw && body_state.raw_format == RawFormat::Json {
            let text = body_state.raw_content.read(cx).value().to_string();
            count_lines(&text)
        } else {
            1
        }
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

    /// 开始拖拽响应编辑器调整大小
    pub fn start_response_editor_drag(&mut self, start_y: f32) {
        self.response_editor_dragging = true;
        self.response_editor_start_y = start_y;
    }

    /// 更新响应编辑器拖拽位置
    pub fn update_response_editor_drag(&mut self, current_y: f32) {
        if self.response_editor_dragging {
            let delta_y = current_y - self.response_editor_start_y;
            self.response_editor_height = (self.response_editor_height + delta_y).max(100.0);
            self.response_editor_start_y = current_y;
        }
    }

    /// 结束响应编辑器拖拽
    pub fn end_response_editor_drag(&mut self) {
        self.response_editor_dragging = false;
    }

    /// 格式化 JSON
    pub fn format_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.body_state.format_json(window, cx);
    }

    // ==================== Params 操作 ====================

    /// 添加新的参数行
    pub fn add_param(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
        self.params.push(ParamEntry { key, value, enabled: true });
        self.rebuild_param_subscriptions(window, cx);
        self.sync_params_to_url(window, cx);
        cx.notify();
    }

    /// 删除指定索引的参数行
    pub fn remove_param(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.params.len() {
            self.params.remove(index);
            self.rebuild_param_subscriptions(window, cx);
            self.sync_params_to_url(window, cx);
            cx.notify();
        }
    }

    /// 切换参数启用状态
    pub fn toggle_param(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if index < self.params.len() {
            self.params[index].enabled = !self.params[index].enabled;
            self.sync_params_to_url(_window, cx);
            cx.notify();
        }
    }

    // ==================== Headers 操作 ====================

    /// 添加新的 Header 行
    pub fn add_header(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.headers.push(HeaderEntry::new(window, cx));
        self.rebuild_header_subscriptions(window, cx);
        cx.notify();
    }

    /// 删除指定索引的 Header 行
    /// 根据响应头重建响应头输入状态（用于显示键值对）
    fn rebuild_response_header_inputs(&mut self, headers: &std::collections::HashMap<String, String>, window: &mut Window, cx: &mut Context<Self>) {
        self.response_header_inputs.clear();
        for (k, v) in headers.iter() {
            let key_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(k)
            });
            let val_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(v)
            });
            self.response_header_inputs.push((key_input, val_input));
        }
    }

    pub fn remove_header(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.headers.len() {
            self.headers.remove(index);
            self.rebuild_header_subscriptions(window, cx);
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

    /// 切换响应体软换行
    pub fn toggle_response_soft_wrap(&mut self, cx: &mut Context<Self>) {
        self.response_soft_wrap = !self.response_soft_wrap;
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

    /// 设置响应体 Raw 格式
    pub fn set_response_raw_format(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.response_raw_format = RawFormat::from_index(index);
        cx.notify();
    }

    /// 获取响应体 Raw 格式
    pub fn get_response_raw_format(&self) -> RawFormat {
        self.response_raw_format
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

        self.rebuild_param_subscriptions(window, cx);
        cx.notify();
    }

    /// 更新URL以反映当前的params
    pub fn sync_params_to_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        let new_url = if enabled_params.is_empty() {
            base_url
        } else {
            let query_string: String = enabled_params
                .iter()
                .map(|(key, value, _)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", base_url, query_string)
        };

        // 同步到 url 字段（send_request 使用）
        self.url = new_url.clone();
        // 记录将要写入的 URL，避免后续异步 change 事件触发 parse_url_to_params_internal
        self.last_synced_url = new_url.clone();
        // 同步到 url_input（URL 栏显示）
        self.url_input.update(cx, |state, cx| {
            state.set_value(&new_url, window, cx);
        });

        cx.notify();
    }

    /// 重建所有 param key/value 的输入变化订阅，实现实时 URL 同步
    pub(crate) fn rebuild_param_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._param_input_subs.clear();
        for param in &self.params {
            let key = param.key.clone();
            let value = param.value.clone();
            let key_sub = cx.subscribe_in(&key, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_params_to_url(_window, cx);
                }
            });
            let value_sub = cx.subscribe_in(&value, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_params_to_url(_window, cx);
                }
            });
            self._param_input_subs.push(key_sub);
            self._param_input_subs.push(value_sub);
        }
    }

    /// 重建所有 header key/value 的输入变化订阅，检测 Content-Type 自动切换 Body 类型
    pub(crate) fn rebuild_header_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._header_subs.clear();
        for header in &self.headers {
            let key = header.key.clone();
            let value = header.value.clone();
            let key_sub = cx.subscribe_in(&key, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.auto_detect_body_type_from_headers(_window, cx);
                }
            });
            let value_sub = cx.subscribe_in(&value, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.auto_detect_body_type_from_headers(_window, cx);
                }
            });
            self._header_subs.push(key_sub);
            self._header_subs.push(value_sub);
        }
    }

    /// 根据 Content-Type header 自动切换 Body 类型
    pub(crate) fn auto_detect_body_type_from_headers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content_type = self.headers.iter().find_map(|h| {
            let key = h.key.read(cx).value();
            if key.trim().eq_ignore_ascii_case("content-type") {
                let val = h.value.read(cx).value().to_string();
                if !val.is_empty() {
                    Some(val)
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(ct) = content_type {
            let ct_lower = ct.to_lowercase();
            if ct_lower.contains("json") || ct_lower.contains("xml")
                || ct_lower.contains("html") || ct_lower.contains("text")
                || ct_lower.contains("javascript")
            {
                // 切换到 Raw
                if self.body_state.body_type != BodyType::Raw {
                    self.body_state.body_type = BodyType::Raw;
                    let bt_idx = BodyType::Raw.to_index();
                    self.body_type_select.update(cx, |state, cx| {
                        state.set_selected_index(Some(IndexPath::new(bt_idx)), window, cx);
                    });
                }
                // 检测格式
                let detected = RawFormat::detect(Some(&ct), "");
                if self.body_state.raw_format != detected {
                    self.body_state.raw_format = detected;
                    let rf_idx = detected.to_index();
                    self.raw_format_select.update(cx, |state, cx| {
                        state.set_selected_index(Some(IndexPath::new(rf_idx)), window, cx);
                    });
                }
                cx.notify();
            }
        }
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
            let body_owned = body.clone();
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
            name: self.t("sidebar.new_request"),
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
                self.t("sidebar.new_request")
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

    /// 加载已保存的请求到当前表单
    fn load_saved_request(
        &mut self,
        _id: &str,
        method: &str,
        url: &str,
        _name: &str,
        headers: Option<&str>,
        body: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let url_owned = url.to_string();
        let body_owned = body.unwrap_or("").to_string();
        self.method = method.to_string();
        self.url = url_owned.clone();
        self.is_importing_curl = true;

        self.url_input.update(cx, |state, cx| {
            state.set_value(&url_owned, window, cx);
        });

        let method_upper = method.to_uppercase();
        let method_idx = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
            .iter()
            .position(|&m| m == method_upper.as_str())
            .unwrap_or(0);
        self.method_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(method_idx)), window, cx);
        });

        self.headers.clear();
        if let Some(headers_text) = headers {
            for line in headers_text.lines() {
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    if !key.is_empty() {
                        self.headers.push(HeaderEntry::new(window, cx));
                        let last = self.headers.len() - 1;
                        self.headers[last].key.update(cx, |state, cx| {
                            state.set_value(&key, window, cx);
                        });
                        self.headers[last].value.update(cx, |state, cx| {
                            state.set_value(&value, window, cx);
                        });
                    }
                }
            }
        }

        // 恢复请求体
        if !body_owned.is_empty() {
            // 从 Content-Type header 检测格式
            let content_type = headers.and_then(|h| {
                h.lines().find_map(|line| {
                    let (k, v) = line.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("content-type") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
            });
            self.body_state.body_type = BodyType::Raw;
            self.body_state.raw_format =
                RawFormat::detect(content_type.as_deref(), &body_owned);
            // 同步 body_type_select
            let bt_idx = BodyType::Raw.to_index();
            self.body_type_select.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(bt_idx)), window, cx);
            });
            // 同步 raw_format_select
            let rf_idx = self.body_state.raw_format.to_index();
            self.raw_format_select.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(rf_idx)), window, cx);
            });
            // JSON 格式化写入
            let formatted = RawFormat::Json.format_body(&body_owned);
            self.body_state.raw_content.update(cx, |state, cx| {
                state.set_value(&formatted, window, cx);
            });
            // 原始内容写入其他格式编辑器
            self.body_state.raw_content_xml.update(cx, |state, cx| {
                state.set_value(&body_owned, window, cx);
            });
            self.body_state.raw_content_text.update(cx, |state, cx| {
                state.set_value(&body_owned, window, cx);
            });
            self.body_state.raw_content_html.update(cx, |state, cx| {
                state.set_value(&body_owned, window, cx);
            });
        } else {
            self.body_state.body_type = BodyType::None;
            let bt_idx = BodyType::None.to_index();
            self.body_type_select.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(bt_idx)), window, cx);
            });
            self.body_state.raw_content.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
        }

        // 从 URL 解析 query params
        self.params.clear();
        if let Some(query_start) = url_owned.find('?') {
            let query_string = &url_owned[query_start + 1..];
            for param in query_string.split('&') {
                if let Some(eq_pos) = param.find('=') {
                    let key = urlencoding::decode(&param[..eq_pos])
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| param[..eq_pos].to_string());
                    let value = urlencoding::decode(&param[eq_pos + 1..])
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| param[eq_pos + 1..].to_string());
                    let key_entity = cx.new(|cx| InputState::new(window, cx).default_value(&key));
                    let value_entity = cx.new(|cx| InputState::new(window, cx).default_value(&value));
                    self.params.push(ParamEntry {
                        key: key_entity,
                        value: value_entity,
                        enabled: true,
                    });
                }
            }
        }
        self.rebuild_param_subscriptions(window, cx);
        self.rebuild_header_subscriptions(window, cx);
        self.auto_detect_body_type_from_headers(window, cx);

        self.last_synced_url = self.url_input.read(cx).value().to_string();
        self.is_importing_curl = false;
        cx.notify();
    }

    /// 激活指定环境
    fn activate_environment(&mut self, env_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        // Single lock acquisition to avoid re-entrant deadlock
        {
            let mut app = self.app_state.lock().unwrap();
            if let Err(e) = app.db.set_active_environment(env_id) {
                log::error!("设置活跃环境失败: {}", e);
                return;
            }
            if let Ok(Some(env)) = app.db.get_active_environment() {
                if let Err(e) = app.env_manager.load_from_json(&env.variables) {
                    log::warn!("加载环境变量失败: {}", e);
                }
            }
            // Sync the HTTP client's env_manager so variable replacement uses current env
            let updated_env = app.env_manager.clone();
            app.http_client.set_env_manager(updated_env);
            self.environments = app.db.get_environments().unwrap_or_default();
            self.active_environment_name = self.environments.iter()
                .find(|e| e.is_active)
                .map(|e| e.name.clone());
        }
        cx.notify();
    }

    /// 从数据库刷新环境列表
    fn load_environments(&mut self) {
        self.environments = self.app_state.lock().unwrap().db.get_environments().unwrap_or_default();
        self.active_environment_name = self.environments.iter()
            .find(|e| e.is_active)
            .map(|e| e.name.clone());
    }

    /// 打开创建/编辑环境对话框
    fn open_environment_dialog(
        &mut self,
        env_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let env_data = if let Some(ref id) = env_id {
            self.environments.iter().find(|e| e.id == *id).cloned()
        } else {
            None
        };
        self.env_dialog_state.lock().unwrap().load_from(
            env_data.as_ref(),
            window,
            cx,
        );
        cx.notify();
    }

    /// 打开全局变量编辑对话框
    fn open_global_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let globals = self.app_state.lock().unwrap().env_manager.get_all_globals();
        self.env_dialog_state.lock().unwrap()
            .open_global(&globals, window, cx);
        cx.notify();
    }

    /// 获取当前活跃环境ID
    pub(crate) fn active_env_id(&self) -> Option<String> {
        self.environments.iter().find(|e| e.is_active).map(|e| e.id.clone())
    }

    /// 删除环境
    fn delete_environment(&mut self, env_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = self.app_state.lock().unwrap().db.delete_environment(env_id) {
            log::error!("删除环境失败: {}", e);
            return;
        }
        self.load_environments();
        cx.notify();
    }

    /// 切换主题 — 更新 AppState、gpui_component 主题并持久化
    fn switch_theme(&mut self, theme: &str, cx: &mut Context<Self>) {
        self.app_state.lock().unwrap().set_theme(theme);

        let mode = match theme {
            "light" => gpui_component::theme::ThemeMode::Light,
            _ => gpui_component::theme::ThemeMode::Dark,
        };
        gpui_component::theme::Theme::change(mode, None, cx);

        self.show_settings_popover = false;
        cx.notify();
    }

    /// 保存当前请求到收藏
    pub fn save_current_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.url_input.read(cx).value().to_string();
        let method = self.method_select.read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "GET".to_string());
        if url.trim().is_empty() {
            return;
        }
        // 从URL提取默认title
        let default_name = url.strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(&url)
            .trim_end_matches('/');
        let short_name = if default_name.len() > 50 {
            format!("{}…", &default_name[..50])
        } else {
            default_name.to_string()
        };
        let headers_text = self.headers.iter()
            .filter(|h| h.enabled)
            .map(|h| {
                let k = h.key.read(cx).value().to_string();
                let v = h.value.read(cx).value().to_string();
                format!("{}: {}", k, v)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let body_text = self.body_state.raw_content.read(cx).value().to_string();
        let saved = SavedRequest {
            id: uuid::Uuid::new_v4().to_string(),
            name: short_name.clone(),
            method,
            url,
            headers: if headers_text.is_empty() { None } else { Some(headers_text) },
            body: if body_text.is_empty() { None } else { Some(body_text) },
            description: None,
            folder_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.save_request_dialog.lock().unwrap()
            .open(short_name, saved, window, cx);
        cx.notify();
    }

    /// 展开/折叠文件夹
    pub fn toggle_folder_expand(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.expanded_folders.contains(folder_id) {
            self.expanded_folders.remove(folder_id);
        } else {
            self.expanded_folders.insert(folder_id.to_string());
        }
        self.rebuild_collections();
        cx.notify();
    }

    /// 重建集合树
    fn rebuild_collections(&mut self) {
        self.collection_items = crate::ui::sidebar::build_collection_tree(
            &self.folders,
            &self.saved_requests,
            &self.expanded_folders,
        );
        log::debug!("集合树已重建: {} 个项目 ({} 个文件夹, {} 个请求)",
            self.collection_items.len(), self.folders.len(), self.saved_requests.len());
    }

    /// 重新加载集合数据（从数据库）
    fn reload_collections(&mut self) {
        if let Ok(app) = self.app_state.lock() {
            self.saved_requests = app.db.get_saved_requests().unwrap_or_default();
            self.folders = app.db.get_folders().unwrap_or_default();
        }
        self.rebuild_collections();
    }

    /// 打开文件夹创建对话框
    pub fn open_folder_dialog(&mut self, parent_id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("打开文件夹对话框, parent_id: {:?}", parent_id);
        self.folder_dialog_state.lock().unwrap().open_for_create(parent_id, window, cx);
        cx.notify();
    }

    /// 打开文件夹重命名对话框
    pub fn open_folder_edit_dialog(&mut self, folder_id: String, current_name: String, window: &mut Window, cx: &mut Context<Self>) {
        let parent_id = self.folders.iter()
            .find(|f| f.id == folder_id)
            .and_then(|f| f.parent_id.clone());
        self.folder_dialog_state.lock().unwrap()
            .open_for_edit(folder_id, current_name, parent_id, window, cx);
        cx.notify();
    }

    /// 重命名收藏请求
    pub fn open_request_rename_dialog(&mut self, request_id: &str, current_name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.folder_dialog_state.lock().unwrap()
            .open_for_request_rename(request_id.to_string(), current_name.to_string(), window, cx);
        cx.notify();
    }

    /// 删除文件夹
    pub fn delete_folder_from_sidebar(&mut self, folder_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Ok(app) = self.app_state.lock() {
            let _ = app.db.delete_folder_cascade(folder_id);
        }
        self.reload_collections();
        cx.notify();
    }

    /// 删除收藏请求
    pub fn delete_saved_request_from_sidebar(&mut self, request_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Ok(app) = self.app_state.lock() {
            let _ = app.db.delete_saved_request(request_id);
        }
        self.reload_collections();
        cx.notify();
    }

    /// 打开移动对话框
    pub fn open_move_dialog(&mut self, item_id: &str, is_folder: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.move_dialog_state.lock().unwrap().open(item_id.to_string(), is_folder);
        cx.notify();
    }

    /// 通过 ID 加载收藏请求（用于树节点点击）
    pub fn load_saved_request_by_id(
        &mut self,
        request_id: &str,
        method: &str,
        url: &str,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let found = self.app_state.lock().ok().and_then(|app| {
            app.db.get_saved_request_by_id(request_id).ok().flatten().map(|req| {
                (req.id, req.method, req.url, req.name, req.headers, req.body)
            })
        });
        if let Some((id, m, u, n, h, b)) = found {
            self.load_saved_request(&id, &m, &u, &n, h.as_deref(), b.as_deref(), window, cx);
            return;
        }
        // fallback: use provided values
        self.load_saved_request(request_id, method, url, name, None, None, window, cx);
    }

    /// 检查所有对话框刷新标志
    fn check_dialog_refresh_flags(&mut self) {
        // 环境变量对话框
        if self.env_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 环境变量对话框标记");
            self.load_environments();
            self.saved_requests = self.app_state.lock().unwrap().db.get_saved_requests().unwrap_or_default();
            self.folders = self.app_state.lock().unwrap().db.get_folders().unwrap_or_default();
            self.env_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 文件夹对话框
        if self.folder_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 文件夹对话框标记");
            self.folders = self.app_state.lock().unwrap().db.get_folders().unwrap_or_default();
            self.saved_requests = self.app_state.lock().unwrap().db.get_saved_requests().unwrap_or_default();
            self.folder_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 移动对话框
        if self.move_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 移动对话框标记");
            self.folders = self.app_state.lock().unwrap().db.get_folders().unwrap_or_default();
            self.saved_requests = self.app_state.lock().unwrap().db.get_saved_requests().unwrap_or_default();
            self.move_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 保存到收藏夹对话框
        if self.save_request_dialog.lock().unwrap().needs_refresh {
            log::debug!("刷新: 保存到收藏夹对话框标记");
            self.saved_requests = self.app_state.lock().unwrap().db.get_saved_requests().unwrap_or_default();
            self.folders = self.app_state.lock().unwrap().db.get_folders().unwrap_or_default();
            self.save_request_dialog.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 集合面板
        if self.needs_collections_refresh {
            log::debug!("刷新: 重建集合树 (needs_collections_refresh=true)");
            self.rebuild_collections();
            self.needs_collections_refresh = false;
        }
    }
}

/// 设置浮层面板 — 在侧边栏 logo 下方展开
fn settings_popover(
    this: &mut MainView,
    cx: &mut Context<MainView>,
    theme: &Theme,
) -> gpui::Div {
    let current_lang = this.app_state.lock().unwrap().config.general.language.clone();
    let current_theme = this.app_state.lock().unwrap().config.general.theme.clone();
    let auto_save = this.app_state.lock().unwrap().config.general.auto_save;
    let proxy_enabled = this.app_state.lock().unwrap().config.proxy.enabled;
    let proxy_url = this.app_state.lock().unwrap().config.proxy.url.clone();

    div()
        .px_3()
        .py_3()
        .border_b(px(1.0))
        .border_color(theme.muted_background)
        .bg(theme.background)
        .flex_col()
        .gap_3()
        .children([
            // === 语言 ===
            section_label("语言 / Language", theme),
            div()
                .flex()
                .gap_2()
                .child(setting_option_btn(
                    "中文",
                    "lang-zh",
                    current_lang == "zh-CN",
                    theme,
                    cx,
                    |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        this.app_state.lock().unwrap().switch_language("zh-CN");
                        cx.notify();
                    },
                ))
                .child(setting_option_btn(
                    "English",
                    "lang-en",
                    current_lang == "en-US",
                    theme,
                    cx,
                    |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        this.app_state.lock().unwrap().switch_language("en-US");
                        cx.notify();
                    },
                )),
            // === 主题 ===
            section_label("主题 / Theme", theme),
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(setting_option_btn(
                    "暗色",
                    "theme-dark",
                    current_theme == "dark",
                    theme,
                    cx,
                    |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        this.switch_theme("dark", cx);
                    },
                ))
                .child(setting_option_btn(
                    "浅色",
                    "theme-light",
                    current_theme == "light",
                    theme,
                    cx,
                    |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        this.switch_theme("light", cx);
                    },
                ))
                .child(setting_option_btn(
                    "暖色",
                    "theme-sepia",
                    current_theme == "sepia",
                    theme,
                    cx,
                    |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        this.switch_theme("sepia", cx);
                    },
                )),
            // === 常规 ===
            section_label("常规 / General", theme),
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child("自动保存 / Auto Save"),
                )
                .child(
                    toggle_switch("auto-save", auto_save, theme, cx,
                        |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                            let new_val = !this.app_state.lock().unwrap().config.general.auto_save;
                            this.app_state.lock().unwrap().config.general.auto_save = new_val;
                            let _ = this.app_state.lock().unwrap().config.save();
                            cx.notify();
                        },
                    ),
                ),
            // === 代理 ===
            section_label("代理 / Proxy", theme),
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child("启用代理 / Enable"),
                )
                .child(
                    toggle_switch("proxy-enabled", proxy_enabled, theme, cx,
                        |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                            let new_val = !this.app_state.lock().unwrap().config.proxy.enabled;
                            this.app_state.lock().unwrap().config.proxy.enabled = new_val;
                            let _ = this.app_state.lock().unwrap().config.save();
                            cx.notify();
                        },
                    ),
                ),
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("代理地址: {}", if proxy_url.is_empty() { "(未设置)" } else { &proxy_url })),
        ])
}

fn section_label(label: &'static str, theme: &Theme) -> gpui::Div {
    div()
        .text_xs()
        .font_semibold()
        .text_color(theme.muted_foreground)
        .child(label)
}

fn setting_option_btn(
    label: &'static str,
    id: &'static str,
    active: bool,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_toggle: impl Fn(&mut MainView, &MouseDownEvent, &mut Window, &mut Context<MainView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .text_sm()
        .cursor_pointer()
        .min_w(px(70.0))
        .px_3()
        .py_1()
        .rounded_sm()
        .bg(if active { theme.accent } else { theme.input_background })
        .text_color(if active { theme.accent_foreground } else { theme.foreground })
        .on_mouse_down(MouseButton::Left, cx.listener(on_toggle))
        .child(label)
}

fn toggle_switch(
    _id: &'static str,
    value: bool,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_toggle: impl Fn(&mut MainView, &MouseDownEvent, &mut Window, &mut Context<MainView>) + 'static,
) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .bg(if value { theme.success } else { theme.muted_background })
        .text_color(if value { theme.accent_foreground } else { theme.foreground })
        .on_mouse_down(MouseButton::Left, cx.listener(on_toggle))
        .child(if value { "ON" } else { "OFF" })
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
        // 检查所有对话框保存后是否需要刷新
        self.check_dialog_refresh_flags();
        let saved_requests = self.saved_requests.clone();
        let folders = self.folders.clone();
        let collection_items = self.collection_items.clone();
        let environments = self.environments.clone();
        let show_close = self.request_tabs.len() > 1;
        let theme = Theme::from_str(&self.app_state.lock().unwrap().theme_name);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .relative()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                if event.keystroke.modifiers.control {
                    match event.keystroke.key.as_str() {
                        "enter" => {
                            let url = this.url_input.read(cx).value().to_string();
                            if !url.trim().is_empty() {
                                let method = this.method_select.read(cx).selected_value()
                                    .unwrap_or(&SharedString::from("GET")).clone();
                                this.url = url;
                                this.method = method.to_string();
                                this.send_request(window, cx);
                            }
                        }
                        "n" => {
                            this.add_tab(window, cx);
                        }
                        "w" => {
                            if this.request_tabs.len() > 1 {
                                let tab_idx = this.active_tab;
                                this.close_tab(tab_idx, window, cx);
                            }
                        }
                        _ => {}
                    }
                }
            }))
            .children([
                // ==================== 主体布局 ====================
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .children([
                        // ==================== 侧边栏 ====================
                        div()
                            .relative()
                            .w(sidebar_width)
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(theme.background)
                            .border_r(px(1.0))
                            .border_color(theme.muted_background)
                            .children([
                                // Logo行
                                div()
                                    .h(px(48.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .border_b(px(1.0))
                                    .border_color(theme.muted_background)
                                    .children([
                                        div().text_color(rgb(0xf97316)).font_semibold().child("ApiPost"),
                                        // 设置按钮
                                        div()
                                            .h(px(28.0))
                                            .w(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .bg(if self.show_settings_popover { theme.muted_background } else { theme.input_background })
                                            .hover(|s| s.bg(theme.muted_background))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.show_settings_popover = !this.show_settings_popover;
                                                cx.notify();
                                            }))
                                            .child(
                                                Icon::new(IconName::Settings).small()
                                            ),
                                    ]),
                                // 标签页按钮
                                div()
                                    .flex()
                                    .flex_row()
                                    .h(px(40.0))
                                    .children([
                                        div()
                                            .id("sidebar-collections-tab") // 收藏夹
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Collections { theme.muted_background } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::Collections { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Collections, cx);
                                            }))
                                            .child(Icon::new(IconName::FolderClosed).small()),
                                        div()
                                            .id("sidebar-history") // 历史记录
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::History { theme.muted_background } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::History { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::History, cx);
                                            }))
                                            .child(Icon::new(IconName::GalleryVerticalEnd).small()),
                                        div()
                                            .id("sidebar-env-tab")
                                            .w(px(48.0))
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Environments { theme.muted_background } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::Environments { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Environments, cx);
                                            }))
                                            .child(Icon::new(IconName::Globe).small()),
                                    ]),
                                // 侧边栏内容
                                if !self.sidebar_collapsed {
                                    div()
                                        .flex()
                                        .flex_1()
                                        .h(px(600.0))
                                        .overflow_hidden()
                                        .children([
                                            if sidebar_tab == SidebarTab::History { // 历史记录
                                                if history.is_empty() {
                                                    div()
                                                        .id("history-empty")
                                                        .p_4()
                                                        .text_sm()
                                                        .text_color(theme.muted_foreground)
                                                        .child(self.t("ui.no_history"))
                                                } else {
                                                    div()
                                                        .id("history-list")
                                                        .flex_col()
                                                        .gap_1()
                                                        .overflow_y_scroll()
                                                        .p_2()
                                                        .children(history.iter().map(|entry| {
                                                            let method_clr = method_color(&entry.method);
                                                            let entry_url = entry.url.clone();
                                                            let entry_method = entry.method.clone();
                                                            let entry_clone = entry.clone();
                                                            let entry_response_body = entry.response_body.clone();
                                                            let entry_response_headers = entry.response_headers.clone();
                                                            let entry_response_time_ms = entry.response_time_ms;
                                                            let entry_response_size = entry.response_size.or_else(|| entry.response_body.as_ref().map(|b| b.len() as i64));
                                                            let display_method = entry.method.clone();
                                                            let display_url = entry.url.clone();
                                                            div()
                                                                .flex_col()
                                                                .gap_1()
                                                                .p_2()
                                                                .rounded_md()
                                                                .cursor_pointer()
                                                                .hover(|s| s.bg(theme.code_background)).bg(theme.muted_background)
                                                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    this.load_saved_request(
                                                                        &entry_clone.id,
                                                                        &entry_clone.method,
                                                                        &entry_clone.url,
                                                                        "",
                                                                        entry_clone.headers.as_deref(),
                                                                        entry_clone.body.as_deref(),
                                                                        _window,
                                                                        cx,
                                                                    );

                                                                    if let Some(status) = entry_clone.response_status {
                                                                        let resp_body = entry_response_body.clone().unwrap_or_default();
                                                                        let resp_headers: std::collections::HashMap<String, String> = entry_response_headers.as_ref().and_then(|h| serde_json::from_str(h).ok()).unwrap_or_default();
                                                                        let content_type = resp_headers.get("content-type").cloned();
                                                                        // 在 resp_headers 被移动前重建 header inputs
                                                                        this.rebuild_response_header_inputs(&resp_headers, _window, cx);
                                                                        let response = HttpResponse {
                                                                            status: status as u16,
                                                                            headers: resp_headers,
                                                                            body: resp_body.clone(),
                                                                            time_ms: entry_response_time_ms.unwrap_or(0),
                                                                            size_bytes: entry_response_size.unwrap_or(0),
                                                                            cookies: Vec::new(),
                                                                        };
                                                                        this.response = Some(response);
                                                                        this.response_raw_format = RawFormat::detect(content_type.as_deref(), &resp_body);
                                                                        let json_body = RawFormat::Json.format_body(&resp_body);
                                                                        this.response_input.update(cx, |state, cx| {
                                                                            state.set_value(&json_body, _window, cx);
                                                                        });
                                                                        this.response_xml_input.update(cx, |state, cx| {
                                                                            state.set_value(&resp_body, _window, cx);
                                                                        });
                                                                        this.response_text_input.update(cx, |state, cx| {
                                                                            state.set_value(&resp_body, _window, cx);
                                                                        });
                                                                        this.response_html_input.update(cx, |state, cx| {
                                                                            state.set_value(&resp_body, _window, cx);
                                                                        });
                                                                    } else {
                                                                        this.response = None;
                                                                        this.response_input.update(cx, |state, cx| {
                                                                            state.set_value("", _window, cx);
                                                                        });
                                                                        this.response_xml_input.update(cx, |state, cx| {
                                                                            state.set_value("", _window, cx);
                                                                        });
                                                                        this.response_text_input.update(cx, |state, cx| {
                                                                            state.set_value("", _window, cx);
                                                                        });
                                                                        this.response_html_input.update(cx, |state, cx| {
                                                                            state.set_value("", _window, cx);
                                                                        });
                                                                        this.response_header_inputs.clear();
                                                                    }
                                                                }))
                                                                .children([
                                                                    div().flex().items_center().gap_2().children([
                                                                        div().px_1().py_px().rounded_sm().bg(rgb(method_clr))
                                                                            .text_xs().text_color(theme.accent_foreground)
                                                                            .child(display_method),
                                                                        div().flex_1().text_ellipsis().text_xs().text_color(theme.foreground)
                                                                            .child(display_url),
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
                                            } else if sidebar_tab == SidebarTab::Collections { // 收藏夹
                                                log::debug!("渲染收藏夹面板: collection_items={}, saved_requests={}, folders={}",
                                                    collection_items.len(), saved_requests.len(), folders.len());
                                                div()
                                                    .id("sidebar-collections")
                                                    .relative()
                                                    .flex_col()
                                                    .flex_1()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                        this.context_menu_target = None;
                                                        this.env_menu_target = None;
                                                        cx.notify();
                                                    }))
                                                    .on_mouse_move(cx.listener(|this, _: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                        this.hovered_item_name = None;
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        // 标题栏：新建文件夹按钮
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .items_center()
                                                            .justify_between()
                                                            .px_2()
                                                            .py_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(theme.muted_foreground)
                                                                    .child(format!("收藏夹 ({})", collection_items.len())),
                                                            )
                                                            .child(
                                                                Button::new("add-folder-btn")
                                                                    .icon(IconName::Plus)
                                                                    .xsmall()
                                                                    .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                                        this.open_folder_dialog(None, window, cx);
                                                                    })),
                                                            ),
                                                    )
                                                    .child(
                                                        if collection_items.is_empty() && saved_requests.is_empty() && folders.is_empty() {
                                                            div()
                                                                .id("collections-empty")
                                                                .p_2()
                                                                .text_sm()
                                                                .text_color(theme.muted_foreground)
                                                                .child(self.t("sidebar.collections_empty"))
                                                                .into_any_element()
                                                        } else {
                                                            div()
                                                                .id("sidebar-collections-scroll")
                                                                .flex_1()
                                                                .overflow_y_scroll()
                                                                .overflow_x_hidden()
                                                                .p_2()
                                                                .child(render_collection_panel(&collection_items, &self.context_menu_target, &self.hovered_item_name, cx, &theme, &self.app_state, self.needs_drop_refresh.clone()))
                                                                .into_any_element()
                                                        }
                                                    )
                                            } else {
                                                div()
                                                    .id("sidebar-environments")
                                                    .relative()
                                                    .flex_col()
                                                    .flex_1()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                        this.context_menu_target = None;
                                                        this.env_menu_target = None;
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .items_center()
                                                            .justify_between()
                                                            .px_2()
                                                            .py_1()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_1()
                                                                    .child(Icon::new(IconName::Globe).xsmall().text_color(theme.accent))
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .text_color(theme.muted_foreground)
                                                                            .child(self.t("sidebar.env")),
                                                                    ),
                                                            )
                                                            .child({
                                                                Button::new("add-env-btn")
                                                                    .icon(IconName::Plus)
                                                                    .xsmall()
                                                                    .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                                        this.open_environment_dialog(None, window, cx);
                                                                    }))
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .id("sidebar-env-scroll")
                                                            .flex_1()
                                                            .overflow_y_scroll()
                                                            .overflow_x_hidden()
                                                            .child(
                                                        if environments.is_empty() {
                                                            div()
                                                                .id("sidebar-env-empty")
                                                                .px_2()
                                                                .py_4()
                                                                .flex()
                                                                .justify_center()
                                                                .text_xs()
                                                                .text_color(theme.muted_foreground)
                                                                .child(self.t("env.no_env"))
                                                        } else {
                                                            div()
                                                                .id("sidebar-env-list")
                                                                .flex_col()
                                                                .w_full()
                                                                .gap_1()
                                                                .py_1()
                                                                .pl_3()
                                                                .pr_1()
                                                                .child(
                                                                    // 全局变量行（首行）
                                                                    div()
                                                                        .w_full()
                                                                        .relative()
                                                                        .flex()
                                                                        .flex_row()
                                                                        .items_center()
                                                                        .py_1()
                                                                        .px_1()
                                                                        .gap_1()
                                                                        .rounded_sm()
                                                                        .cursor_pointer()
                                                                        .hover(|s| s.bg(theme.code_background))
                                                                        .bg(theme.background)
                                                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                                            this.open_global_dialog(window, cx);
                                                                        }))
                                                                        .child(Icon::new(IconName::Star).xsmall().text_color(theme.accent))
                                                                        .child(
                                                                            div().flex_1().text_sm().text_color(theme.foreground).child(self.t("sidebar.global_vars"))
                                                                        ),
                                                                )
                                                                .children(environments.iter().map(|env| {
                                                                    let env_id = env.id.clone();
                                                                    let env_name = env.name.clone();
                                                                    let display_name = if env_name.chars().count() > 12 {
                                                                        format!("{}...", env_name.chars().take(12).collect::<String>())
                                                                    } else {
                                                                        env_name.clone()
                                                                    };
                                                                    let env_id_edit = env.id.clone();
                                                                    let env_id_del = env.id.clone();
                                                                    let is_active = env.is_active;
                                                                    div()
                                                                        .w_full()
                                                                        .relative()
                                                                        .flex()
                                                                        .flex_row()
                                                                        .items_center()
                                                                        .px_1()
                                                                        .py_1()
                                                                        .pr(px(28.0))
                                                                        .rounded_md()
                                                                        .cursor_pointer()
                                                                        .hover(|s| s.bg(theme.code_background))
                                                                        .bg(if is_active { theme.muted_background } else { theme.background })
                                                                        .children([
                                                                            div()
                                                                                .w(px(7.0))
                                                                                .h(px(7.0))
                                                                                .rounded_full()
                                                                                .flex_shrink_0()
                                                                                .bg(if is_active { theme.success } else { theme.muted_foreground }),
                                                                            div()
                                                                                .text_sm()
                                                                                .text_color(if is_active { theme.foreground } else { theme.muted_foreground })
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                                                    this.activate_environment(&env_id, window, cx);
                                                                                }))
                                                                                .child(display_name),
                                                                        ])
                                                                        .child(
                                                                            div()
                                                                                .absolute()
                                                                                .right(px(2.0))
                                                                                .top(px(0.0))
                                                                                .h_full()
                                                                                .flex().items_center()
                                                                                .px(px(2.0))
                                                                                .bg(theme.background)
                                                                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                                                                .child(
                                                                                    div()
                                                                                        .w(px(24.0)).h(px(24.0))
                                                                                        .flex().items_center().justify_center()
                                                                                        .rounded_sm()
                                                                                        .cursor_pointer()
                                                                                        .hover(|s| s.bg(theme.muted_background))
                                                                                        .on_mouse_down(MouseButton::Left, cx.listener({
                                                                                            let id = env_id_edit.clone();
                                                                                            move |this, e: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                                cx.stop_propagation();
                                                                                                this.env_menu_pos = Some((e.position.x.into(), e.position.y.into()));
                                                                                                this.env_menu_target = if this.env_menu_target.as_ref() == Some(&id) {
                                                                                                    None
                                                                                                } else {
                                                                                                    Some(id.clone())
                                                                                                };
                                                                                                cx.notify();
                                                                                            }
                                                                                        }))
                                                                                        .child(
                                                                                            Icon::new(IconName::Ellipsis).xsmall().text_color(theme.muted_foreground),
                                                                                        ),
                                                                                ),
                                                                        )
                                                                }))
                                                        }
                                                    )
                                                    )
                                            },
                                        ])
                                } else {
                                    div().flex_1()
                                },
                                // 侧边栏抽屉 折叠/展开按钮
                                div()
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.code_background))
                                    .border_t(px(1.0))
                                    .border_color(theme.muted_background)
                                    .text_color(theme.muted_foreground)
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        this.toggle_sidebar(cx);
                                    }))
                                    .text_color(theme.muted_foreground)
                                    .child(if self.sidebar_collapsed {
                                        Icon::new(IconName::PanelLeftOpen).small()
                                    } else {
                                        Icon::new(IconName::PanelLeftClose).small()
                                    }),
                            // 设置浮层面板（绝对定位、最后渲染以确保在最上层）
                            if self.show_settings_popover {
                                settings_popover(self, cx, &theme)
                                    .absolute()
                                    .top(px(48.0))
                                    .left(px(0.0))
                                    .w(px(280.0))
                                    .shadow_md()
                            } else {
                                div()
                            },
                            ]),
                        // ==================== 主工作区 ====================
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                this.context_menu_target = None;
                                this.env_menu_target = None;
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
                                    .bg(theme.code_background)
                                    .border_b(px(1.0))
                                    .border_color(theme.muted_background)
                                    .children(request_tabs.iter().enumerate().map(|(i, tab)| {
                                        let is_active = i == active_tab;
                                        let method_clr = method_color(&tab.method);
                                        let tab_method = tab.method.clone();
                                        // 如果是默认标签名称，使用i18n
                                        let tab_display_name = if tab.name == "新建请求" || tab.name == "New Request" {
                                            self.t("sidebar.new_request")
                                        } else {
                                            tab.name.clone()
                                        };
                                        let show_close = show_close;

                                        div()
                                            .h(px(40.0))
                                            .w(px(150.0))
                                            .pl_3()
                                            .pr_1()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .bg(if is_active { theme.background } else { theme.code_background })
                                            .border_b_2()
                                            .border_b(if is_active { px(2.0) } else { px(0.0) })
                                            .border_color(if is_active { theme.accent } else { theme.muted_background })
                                            .text_xs()
                                            .relative()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .rounded_sm()
                                                    .px_1()
                                                    .py_px()
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(theme.muted_background))
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                        this.switch_tab(i, _window, cx);
                                                    }))
                                                    .children([
                                                        div().px_1().py_px().rounded_sm()
                                                            .bg(rgb(method_clr))
                                                            .text_xs().text_color(theme.accent_foreground)
                                                            .child(tab_method),
                                                        div()
                                                            .text_color(if is_active { theme.accent_foreground } else { theme.muted_foreground })
                                                            .max_w(px(90.0))
                                                            .overflow_hidden()
                                                            .text_ellipsis()
                                                            .child(tab_display_name.clone()),
                                                    ]),
                                            )
                                            .when(show_close, |this| {
                                                this.child(
                                                    div()
                                                        .absolute()
                                                        .top(px(2.0))
                                                        .right(px(2.0))
                                                        .w(px(16.0))
                                                        .h(px(16.0))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .rounded_sm()
                                                        .cursor_pointer()
                                                        .hover(|s| s.bg(theme.muted_background))
                                                        .text_color(theme.muted_foreground)
                                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                            this.close_tab(i, _window, cx);
                                                        }))
                                                        .child(Icon::new(IconName::Close).xsmall()),
                                                )
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
                                            .text_color(theme.muted_foreground)
                                            .hover(|s| s.bg(theme.muted_background).text_color(theme.accent_foreground))
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
                                    .bg(theme.background)
                                    .children([
                                        crate::ui::request::render_url_bar(self, _window, cx).into_any_element(),
                                        // 标签页（全部可点击）
                                        div()
                                            .flex()
                                            .flex_row()
                                            .border_b(px(1.0))
                                            .border_color(theme.muted_background)
                                            .children([
                                                self.builder_tab_button(cx, "request.params", BuilderTab::Params, builder_tab, "builder-params").into_any_element(),
                                                self.builder_tab_button(cx, "request.auth", BuilderTab::Authorization, builder_tab, "builder-auth").into_any_element(),
                                                self.builder_tab_button(cx, "request.headers", BuilderTab::Headers, builder_tab, "builder-headers").into_any_element(),
                                                self.builder_tab_button(cx, "request.body", BuilderTab::Body, builder_tab, "builder-body").into_any_element(),
                                                self.builder_tab_button(cx, "request.pre_request", BuilderTab::PreRequest, builder_tab, "builder-pre-request").into_any_element(),
                                                self.builder_tab_button(cx, "request.tests", BuilderTab::Tests, builder_tab, "builder-tests").into_any_element(),
                                                self.builder_tab_button(cx, "request.settings", BuilderTab::Settings, builder_tab, "builder-settings").into_any_element(),
                                            ])
                                            .into_any_element(),
                                        // 各标签页内容 — 使用提取的组件
                                        match builder_tab {
                                            BuilderTab::Params => crate::ui::request::render_params_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::Authorization => crate::ui::request::render_auth_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::Headers => crate::ui::request::render_headers_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::Body => crate::ui::request::render_body_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::PreRequest => crate::ui::request::render_pre_request_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::Tests => crate::ui::request::render_tests_panel(self, _window, cx).into_any_element(),
                                            BuilderTab::Settings => crate::ui::request::render_settings_panel(self, cx).into_any_element(),
                                        }
                                    ]),
                                // Splitter（可拖拽调整上下区域大小）
                                div()
                                    .h(px(5.0))
                                    .w_full()
                                    .bg(theme.muted_background)
                                    .cursor_row_resize()
                                    .hover(|s| s.bg(theme.accent))
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
                                    .bg(theme.background)
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
                                            .border_color(theme.muted_background)
                                            .children([
                                                self.response_tab_button(cx, "response.body", ResponseTab::Body, response_tab, "response-body"),
                                                self.response_tab_button(cx, "response.cookies", ResponseTab::Cookies, response_tab, "response-cookies"),
                                                self.response_tab_button(cx, "response.headers", ResponseTab::Headers, response_tab, "response-headers"),
                                                self.response_tab_button(cx, "response.test_results", ResponseTab::TestResults, response_tab, "response-test-results"),
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
                                            .text_color(theme.muted_foreground)
                                            .children([
                                                if let Some(err) = error_message {
                                                    div()
                                                        .p_3()
                                                        .rounded_md()
                                                        .bg(rgb(0x3f2020))
                                                        .text_color(theme.error)
                                                        .child(err)
                                                } else if let Some(resp) = response {
                                                    let theme = Theme::from_str(&self.app_state.lock().unwrap().theme_name);
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
                                                                .bg(if (200..300).contains(&resp.status) { theme.success } else { theme.error })
                                                                .text_color(theme.accent_foreground)
                                                                .child(format!("{} {}", resp.status, resp.status_text())),
                                                            // 模式选择按钮（仅 Body tab 显示）
                                                            if response_tab == ResponseTab::Body {
                                                                div()
                                                                    .flex()
                                                                    .flex_row()
                                                                    .gap_2()
                                                                    .children([
                                                                        Button::new("pretty")
                                                                            .min_w(px(70.0))
                                                                            .label(self.t("ui.pretty"))
                                                                            .small()
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
                                                                            .min_w(px(70.0))
                                                                            .label(self.t("ui.raw"))
                                                                            .small()
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
                                                                            .min_w(px(70.0))
                                                                            .label(self.t("ui.preview"))
                                                                            .small()
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
                                                                    ])
                                                            } else {
                                                                div()
                                                            },
                                                            // Time 和 Size 在右边
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .gap_4()
                                                                .children([
                                                                    div().text_color(theme.muted_foreground).child(format!("Time: {}ms", resp.time_ms)),
                                                                    div().text_color(theme.muted_foreground).child(format!("Size: {}", format_size(resp.size_bytes))),
                                                        ])
                                                        ]);

                                                    // 根据视图模式显示内容
                                                    let content: Div = if response_tab == ResponseTab::Headers {
                                                        div()
                                                            .h_full()
                                                            .flex_col()
                                                            .overflow_hidden()
                                                            .bg(theme.code_background)
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_md()
                                                            .p_2()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .flex_row()
                                                                    .gap_2()
                                                                    .mb_1()
                                                                    .children([
                                                                        div().flex_1().text_xs().text_color(theme.muted_foreground).child("Key"),
                                                                        div().flex_1().text_xs().text_color(theme.muted_foreground).child("Value"),
                                                                    ])
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex_col()
                                                                    .gap_1()
                                                                    .overflow_y_scrollbar()
                                                                    .flex_1()
                                                                    .children(self.response_header_inputs.iter().map(|(key_input, val_input)| {
                                                                        div()
                                                                            .flex()
                                                                            .flex_row()
                                                                            .gap_2()
                                                                            .children([
                                                                                div().flex_1().child(Input::new(key_input).small().h(px(28.0)).disabled(true).bg(theme.input_background).text_color(theme.foreground)),
                                                                                div().flex_1().child(Input::new(val_input).small().h(px(28.0)).disabled(true).bg(theme.input_background).text_color(theme.foreground)),
                                                                            ])
                                                                    }))
                                                            )
                                                    } else if response_tab == ResponseTab::Cookies {
                                                        div().flex_1().child(crate::ui::response::response_cookies_viewer(&resp.cookies, &theme))
                                                    } else if response_tab == ResponseTab::TestResults {
                                                        // 测试结果（暂未实现）
                                                        div()
                                                            .flex_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .text_color(theme.muted_foreground)
                                                            .child("Test results not implemented")
                                                    } else {
                                                        match self.body_view_mode {
                                                        BodyViewMode::Pretty => {
                                                            div()
                                                                .h_full()
                                                                .flex_col()
                                                                .overflow_hidden()
                                                                .bg(theme.code_background)
                                                                .border_1()
                                                                .border_color(theme.border)
                                                                .rounded_md()
                                                                .child(
                                                                    Input::new(&self.response_input)
                                                                        .w_full()
                                                                        .h_full(),
                                                                )
                                                        },
                                                        BodyViewMode::Raw => {
                                                            div()
                                                                .h_full()
                                                                .flex_col()
                                                                .overflow_hidden()
                                                                .bg(theme.code_background)
                                                                .border_1()
                                                                .border_color(theme.border)
                                                                .rounded_md()
                                                                .child(
                                                                    Input::new(&self.response_input)
                                                                        .w_full()
                                                                        .h_full(),
                                                                )
                                                        },
                                                        BodyViewMode::Preview => {
                                                            div()
                                                                .flex_1()
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .text_color(theme.muted_foreground)
                                                                .child("Preview mode not implemented")
                                                        },
                                                        }
                                                    };

                                                    {
                                                        let area = div()
                                                            .flex_1()
                                                            .flex_col()
                                                            .overflow_hidden()
                                                            .child(header_row);
                                                        if self.body_view_mode == BodyViewMode::Raw {
                                                            let is_json = self.response_raw_format == RawFormat::Json;
                                                            let is_xml = self.response_raw_format == RawFormat::Xml;
                                                            let is_text = self.response_raw_format == RawFormat::Text;
                                                            let is_html = self.response_raw_format == RawFormat::Html;
                                                            area.child(
                                                                div()
                                                                    .flex()
                                                                    .flex_row()
                                                                    .items_center()
                                                                    .gap_1()
                                                                    .px_2()
                                                                    .py_px()
                                                                    .children([
                                                                        div()
                                                                            .text_xs().cursor_pointer().px_2().py_px().rounded_sm()
                                                                            .bg(if is_json { theme.muted_background } else { theme.input_background })
                                                                            .text_color(if is_json { theme.foreground } else { theme.muted_foreground })
                                                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(0, _window, cx); }))
                                                                            .child("JSON"),
                                                                        div()
                                                                            .text_xs().cursor_pointer().px_2().py_px().rounded_sm()
                                                                            .bg(if is_xml { theme.muted_background } else { theme.input_background })
                                                                            .text_color(if is_xml { theme.foreground } else { theme.muted_foreground })
                                                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(1, _window, cx); }))
                                                                            .child("XML"),
                                                                        div()
                                                                            .text_xs().cursor_pointer().px_2().py_px().rounded_sm()
                                                                            .bg(if is_text { theme.muted_background } else { theme.input_background })
                                                                            .text_color(if is_text { theme.foreground } else { theme.muted_foreground })
                                                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(2, _window, cx); }))
                                                                            .child("Text"),
                                                                        div()
                                                                            .text_xs().cursor_pointer().px_2().py_px().rounded_sm()
                                                                            .bg(if is_html { theme.muted_background } else { theme.input_background })
                                                                            .text_color(if is_html { theme.foreground } else { theme.muted_foreground })
                                                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(3, _window, cx); }))
                                                                            .child("HTML"),
                                                                    ]),
                                                            ).child(content)
                                                        } else {
                                                            area.child(content)
                                                        }
                                                    }
                                                } else {
                                                    div()
                                                        .flex_1()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_color(theme.muted_foreground)
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
                    .bg(theme.muted_background)
                    .border_t(px(1.0))
                    .border_color(theme.muted_background)
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .children([
                        div().flex().items_center().gap_4().children([
                            div().child(
                                self.active_environment_name
                                    .clone()
                                    .unwrap_or_else(|| self.t("env.no_env"))
                            ),
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
            .when(
                self.env_dialog_state.lock().map(|s| s.visible).unwrap_or(false),
                |d| d.child(
                    render_env_dialog_overlay(
                        &self.env_dialog_state,
                        &self.app_state,
                        &self.active_env_id(),
                        &theme,
                        cx.entity_id(),
                        cx,
                    )
                )
            )
            .when(
                self.folder_dialog_state.lock().map(|s| s.visible).unwrap_or(false),
                |d| d.child(
                    render_folder_dialog_overlay(
                        &self.folder_dialog_state,
                        &self.app_state,
                        &theme,
                        cx.entity_id(),
                        cx,
                    )
                )
            )
            .when(
                self.hovered_item_name.is_some(),
                |d| {
                    let name = self.hovered_item_name.clone().unwrap_or_default();
                    let row_y = self.hovered_item_y.unwrap_or(0.0);
                    let x = self.hovered_item_x.unwrap_or(0.0);
                    // 浮层在行下方14px，但如果超出窗口底部则翻转到行上方
                    let tooltip_h = 28.0;
                    let window_h = 800.0; // 估计窗口高度
                    let mut y = row_y + 20.0;
                    if y + tooltip_h > window_h {
                        y = row_y - tooltip_h - 4.0;
                    }
                    d.child(
                        tooltip_popup(&theme, x, y, &name)
                    )
                },
            )
            .when(
                self.context_menu_target.is_some() && self.context_menu_pos.is_some(),
                |d| {
                    let target_id = self.context_menu_target.clone().unwrap_or_default();
                    let (x, y) = self.context_menu_pos.unwrap_or((0.0, 0.0));
                    let mut menu = div();
                    if let Some(folder) = self.folders.iter().find(|f| f.id == target_id) {
                        menu = render_folder_context_menu(&target_id, &folder.name, cx, &theme)
                            .absolute()
                            .left(px((x - 140.0).max(0.0)))
                            .top(px(y + 4.0));
                    } else if let Some(req) = self.saved_requests.iter().find(|r| r.id == target_id) {
                        menu = render_request_context_menu(&target_id, &req.name, cx, &theme)
                            .absolute()
                            .left(px((x - 120.0).max(0.0)))
                            .top(px(y + 4.0));
                    }
                    d.child(menu)
                },
            )
            .when(
                self.env_menu_target.is_some() && self.env_menu_pos.is_some(),
                |d| {
                    let target_id = self.env_menu_target.clone().unwrap_or_default();
                    let (x, y) = self.env_menu_pos.unwrap_or((0.0, 0.0));
                    let env = self.environments.iter().find(|e| e.id == target_id);
                    let mut menu = popup_panel(&theme)
                        .absolute()
                        .left(px((x - 120.0).max(0.0)))
                        .top(px(y + 4.0))
                        .min_w(px(120.0));
                    if let Some(env) = env {
                        let edit_id = env.id.clone();
                        let del_id = env.id.clone();
                        menu = menu
                            .child(
                                div()
                                    .flex().flex_row().items_center().gap_2()
                                    .px_3().py_1p5()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.muted_background))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                                        this.env_menu_target = None;
                                        this.open_environment_dialog(Some(edit_id.clone()), window, cx);
                                        cx.notify();
                                    }))
                                    .child(Icon::new(IconName::Replace).xsmall().text_color(theme.muted_foreground))
                                    .child(div().text_sm().text_color(theme.foreground).child("重命名")),
                            )
                            .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
                            .child(
                                div()
                                    .flex().flex_row().items_center().gap_2()
                                    .px_3().py_1p5()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.error))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                                        this.env_menu_target = None;
                                        this.delete_environment(&del_id, window, cx);
                                        cx.notify();
                                    }))
                                    .child(Icon::new(IconName::Delete).xsmall().text_color(theme.muted_foreground))
                                    .child(div().text_sm().text_color(theme.foreground).child("删除")),
                            );
                    }
                    d.child(menu)
                },
            )
            .when(
                self.move_dialog_state.lock().map(|s| s.visible).unwrap_or(false),
                |d| d.child(
                    render_move_dialog_overlay(
                        &self.move_dialog_state,
                        &self.app_state,
                        &theme,
                        cx.entity_id(),
                        cx,
                    )
                )
            )
            .when(
                self.save_request_dialog.lock().map(|s| s.visible).unwrap_or(false),
                |d| {
                    let dialog = self.save_request_dialog.lock().unwrap();
                    let name_input = dialog.name_input.clone();
                    let entity_id = cx.entity_id();
                    drop(dialog);
                    let save_dialog = self.save_request_dialog.clone();
                    let save_app = self.app_state.clone();
                    d.child(
                        div()
                            .absolute()
                            .top_0().left_0().right_0().bottom_0()
                            .bg(rgba(0x00000055))
                            .flex().items_center().justify_center()
                            .on_mouse_down(MouseButton::Left, |_, _, _| {})
                            .child(
                                div()
                                    .w(px(420.0))
                                    .bg(theme.background)
                                    .rounded_xl()
                                    .border_1().border_color(theme.border)
                                    .shadow_2xl()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .h(px(48.0))
                                            .flex().flex_row().items_center().justify_between()
                                            .px_5()
                                            .bg(theme.muted_background)
                                            .border_b_1().border_color(theme.border)
                                            .child(
                                                div().flex().items_center().gap_2()
                                                    .child(Icon::new(IconName::Star).text_color(theme.accent))
                                                    .child(div().text_sm().font_weight(FontWeight(600.0)).text_color(theme.foreground).child("保存到收藏夹")),
                                            )
                                            .child({
                                                let s = save_dialog.clone();
                                                Button::new("close-save-dialog")
                                                    .icon(IconName::Close).small()
                                                    .text_color(theme.muted_foreground)
                                                    .on_click(move |_, _, cx| {
                                                        if let Ok(mut d) = s.lock() { d.visible = false; d.pending_request = None; }
                                                        cx.notify(entity_id);
                                                    })
                                            }),
                                    )
                                    .child(
                                        div().px_5().py_4().flex_col().gap_4()
                                            .child(
                                                div().flex_col().gap_2()
                                                    .child(div().flex().items_center().gap_1p5()
                                                        .child(Icon::new(IconName::File).small().text_color(theme.accent))
                                                        .child(div().text_xs().font_weight(FontWeight(500.0)).text_color(theme.muted_foreground).child("标题")),
                                                    )
                                                    .child(Input::new(&name_input).h(px(42.0)).w_full().rounded_md().bg(theme.background).text_color(theme.foreground)),
                                            )
                                    )
                                    .child(
                                        div().h(px(48.0)).flex().flex_row().justify_end().items_center().gap_3().px_5()
                                            .border_t_1().border_color(theme.border).bg(theme.muted_background)
                                            .child({
                                                let s = save_dialog.clone();
                                                Button::new("cancel-save-dialog-btn").label("取消")
                                                    .on_click(move |_, _, cx| {
                                                        if let Ok(mut d) = s.lock() { d.visible = false; d.pending_request = None; }
                                                        cx.notify(entity_id);
                                                    })
                                            })
                                            .child({
                                                let s = save_dialog.clone();
                                                let app = save_app.clone();
                                                Button::new("confirm-save-dialog-btn").icon(IconName::Check).label("保存")
                                                    .bg(theme.accent).text_color(rgb(0xffffff)).rounded_md()
                                                    .on_click(move |_, _window, cx| {
                                                        let d = s.lock().unwrap();
                                                        let name = d.name_input.read(cx).value().to_string();
                                                        let name = if name.trim().is_empty() { "Untitled".to_string() } else { name.trim().to_string() };
                                                        if let Some(mut pending) = d.pending_request.clone() {
                                                            pending.name = name;
                                                            let result = app.lock().unwrap().db.save_request(&pending);
                                                            drop(d);
                                                            if let Ok(mut d) = s.lock() {
                                                                d.visible = false;
                                                                d.pending_request = None;
                                                                d.needs_refresh = true;
                                                            }
                                                            if let Err(e) = result {
                                                                log::error!("保存请求失败: {}", e);
                                                            }
                                                        } else {
                                                            drop(d);
                                                        }
                                                        cx.notify(entity_id);
                                                    })
                                            }),
                                    )
                            )
                    )
                }
            )
    }
}
