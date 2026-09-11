//! 主视图模块
//!
//! 应用主界面，包含侧边栏和请求/响应面板

use crate::app::database::{Environment, Folder, HistoryEntry, SavedRequest};
use crate::app::history::CreateHistoryEntry;
use crate::app::HttpResponse;
use crate::http::HttpRequest;
use crate::ui::components::{
    ghost_button, popup_panel, primary_button_sm, section_divider, section_title, segment_button,
    segment_group, tooltip_popup, GAP_M, GAP_S, GAP_XS,
};
use crate::ui::dialogs::{
    render_code_gen_dialog_overlay, render_env_dialog_overlay,
    render_folder_dialog_overlay, render_move_dialog_overlay, CodeGenDialogState,
    EnvDialogState, FolderDialogState, MoveDialogState,
};
use crate::ui::sidebar::CollectionItem;
use crate::ui::sidebar::{
    build_collection_tree, render_collection_panel, render_environment_panel, DragItem,
};
use crate::ui::sidebar::{render_folder_context_menu, render_request_context_menu};
use crate::ui::{
    count_lines, json_editor, ApiKeyLocation, AuthState, AuthType, BodyState, BodyType,
    FormDataEntry, FormDataParamType, FormDataValue, HeaderEntry, RawFormat, RequestSettings,
    SavedFormDataEntry, ScriptState, SettingsInputs, Theme,
};
use gpui::prelude::*;
use gpui::InteractiveElement;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::dialog::{Dialog, DialogHeader, DialogTitle};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::Scrollable;
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{Disableable, Icon, IconName, IndexPath, Sizable, StyledExt, WindowExt};
use std::collections::HashSet;
use smallvec::smallvec;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio;

/// 净化文本用于安全渲染，移除会导致 cosmic-text Bidi 断言失败的问题字符。
fn sanitize_display_text(text: &str) -> String {
    text.chars()
        .filter(|&c| {
            !matches!(
                c,
                '\u{061C}'
                | '\u{200E}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{FEFF}'
                | '\u{2060}'
            ) && (c >= ' ' || c == '\n' || c == '\r' || c == '\t')
        })
        .collect()
}

/// 请求标签的单个标签宽度（px）。标签定宽，标签过多时由标签条横向滚动，而不是压窄标签
const REQUEST_TAB_WIDTH: f32 = 150.0;
/// 请求标签条中相邻标签之间的间距（px），需与 `.gap_px()` 保持一致
const REQUEST_TAB_GAP: f32 = 1.0;
/// “新建标签”按钮宽度（px），固定在标签条右侧、不随标签条一起滚动
const NEW_TAB_BUTTON_WIDTH: f32 = 36.0;
/// 标签条左右滚动按钮宽度（px），仅在标签溢出时显示
const TABS_SCROLL_BUTTON_WIDTH: f32 = 24.0;

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
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 请求标签
#[derive(Clone)]
pub struct RequestTab {
    pub id: usize,
    pub method: String,
    pub url: String,
    pub name: String,
    pub tab_state: TabState,
}

/// 每个标签页的完整状态快照（纯数据，不含 Entity 引用）
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TabState {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub name: String,
    pub body_type: BodyType,
    pub raw_format: RawFormat,
    pub raw_json: String,
    pub raw_xml: String,
    pub raw_text: String,
    pub raw_html: String,
    pub headers: Vec<(String, String, bool)>,
    pub params: Vec<(String, String, bool)>,
    #[serde(default)]
    pub form_data: Vec<SavedFormDataEntry>,
    #[serde(default)]
    pub urlencoded_data: Vec<SavedFormDataEntry>,
    pub auth_type_index: usize,
    pub bearer_token: String,
    pub basic_username: String,
    pub basic_password: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_location: ApiKeyLocation,
    pub settings: RequestSettings,
    // 用 Arc 共享：渲染每帧都要做快照，裸 HttpResponse 会每帧克隆一份响应头 HashMap
    pub response: Option<Arc<HttpResponse>>,
    pub response_raw_format: RawFormat,
    pub builder_tab: BuilderTab,
    pub timeout_secs: String,
    pub retry_count: String,
    #[serde(default)]
    pub pre_request_script: String,
    #[serde(default)]
    pub test_script: String,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            url: String::new(),
            name: String::new(),
            body_type: BodyType::None,
            raw_format: RawFormat::Json,
            raw_json: r#"{"key": "value"}"#.to_string(),
            raw_xml: r#"<root></root>"#.to_string(),
            raw_text: String::new(),
            raw_html: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            form_data: Vec::new(),
            urlencoded_data: Vec::new(),
            auth_type_index: 0,
            bearer_token: String::new(),
            basic_username: String::new(),
            basic_password: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
            settings: RequestSettings::default(),
            response: None,
            response_raw_format: RawFormat::Json,
            builder_tab: BuilderTab::Params,
            timeout_secs: "30".to_string(),
            retry_count: "0".to_string(),
            pre_request_script: String::new(),
            test_script: String::new(),
        }
    }
}

/// 请求构造器标签页
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, PartialEq, Debug)]
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
    /// 翻译字典缓存（从 I18nManager 提取，避免每次翻译都获取 Mutex 锁）
    pub(crate) translations: Arc<crate::i18n::Translations>,
    /// 主题缓存（避免每个面板渲染时重复调用 Theme::from_str）
    pub(crate) cached_theme: Theme,
    /// 响应体高亮缓存：解析 / 美化 / 分词只在响应或主题变化时做一次
    pub(crate) response_highlight: Option<crate::ui::response_highlight::Cache>,
    pub method: String,
    pub url: String,
    // Arc 共享：渲染每帧快照只做引用计数；写入走 Arc::make_mut（渲染期间无写操作，不触发拷贝）
    pub(crate) request_tabs: Arc<Vec<RequestTab>>,
    pub(crate) active_tab: usize,
    /// 请求标签条的横向滚动句柄（标签定宽，标签过多时左右滚动，并让当前标签始终可见）
    pub(crate) tabs_scroll: gpui::ScrollHandle,
    /// 启动/恢复工作区后，需要在首次布局完成时把当前标签滚动到可见区域
    pub(crate) tabs_reveal_pending: bool,
    /// 上次可见的标签条宽度（0 表示尚未布局），用于在窗口/侧边栏宽度变化后重新定位当前标签
    pub(crate) tabs_viewport: f32,
    pub(crate) response: Option<Arc<HttpResponse>>,
    pub(crate) response_tab: ResponseTab,
    pub(crate) body_view_mode: BodyViewMode,
    pub(crate) is_loading: bool,
    pub(crate) loading_frame: u64,
    pub(crate) error_message: Option<String>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) sidebar_tab: SidebarTab,
    pub(crate) show_settings_popover: bool,
    /// 自动更新检查状态（设置面板「关于」区）
    pub(crate) update_status: crate::app::updater::UpdateStatus,
    /// 上次检查时间，用于自动检查节流
    pub(crate) update_checked_at: Option<std::time::Instant>,
    pub(crate) context_menu_target: Option<String>,
    pub(crate) hovered_item_name: Option<String>,
    pub(crate) hovered_item_y: Option<f32>,
    pub(crate) hovered_item_x: Option<f32>,
    pub(crate) context_menu_pos: Option<(f32, f32)>,
    /// 历史记录（放在堆上共享：渲染时 clone 一次只是引用计数 +1，不再深拷贝整份列表）
    pub(crate) history: Arc<Vec<HistoryEntry>>,
    pub(crate) saved_requests: Arc<Vec<crate::app::database::SavedRequest>>,
    pub(crate) folders: Arc<Vec<crate::app::database::Folder>>,
    pub(crate) environments: Arc<Vec<crate::app::database::Environment>>,
    pub(crate) active_environment_name: Option<String>,
    pub(crate) env_dialog_state: Arc<Mutex<EnvDialogState>>,
    pub(crate) folder_dialog_state: Arc<Mutex<FolderDialogState>>,
    pub(crate) move_dialog_state: Arc<Mutex<MoveDialogState>>,
    pub(crate) expanded_folders: HashSet<String>,
    pub(crate) collection_items: Arc<Vec<CollectionItem>>,
    pub(crate) needs_collections_refresh: bool,
    pub(crate) needs_drop_refresh: Arc<AtomicBool>,
    pub(crate) url_input: Entity<InputState>,
    pub(crate) _url_change_sub: gpui::Subscription,
    pub(crate) _param_input_subs: Vec<gpui::Subscription>,
    pub(crate) _header_subs: Vec<gpui::Subscription>,
    pub(crate) _form_data_type_subs: Vec<gpui::Subscription>,
    pub(crate) method_select: Entity<SelectState<Vec<crate::ui::components::MethodItem>>>,
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
    pub(crate) response_pretty_input: Entity<InputState>,
    pub(crate) response_header_inputs: Vec<(Entity<InputState>, Entity<InputState>)>,
    pub(crate) response_raw_format: RawFormat,
    pub(crate) response_raw_format_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) response_soft_wrap: bool,
    pub(crate) next_tab_id: usize,
    pub(crate) splitter_dragging: bool,
    pub(crate) splitter_start_y: f32,
    pub(crate) last_splitter_update_y: f32,
    pub(crate) request_builder_height: f32,
    pub(crate) response_editor_height: f32,
    pub(crate) response_editor_dragging: bool,
    pub(crate) response_editor_start_y: f32,
    pub(crate) save_request_dialog: Arc<Mutex<SaveRequestDialog>>,
    /// 上次 workpace 保存时间（用于节流）
    pub(crate) last_workspace_save: std::time::Instant,
    /// 自动保存后台任务句柄（None = 未启动）
    pub(crate) auto_save_task: Option<gpui::Task<()>>,
    /// 代理地址输入（设置弹窗中使用）
    pub(crate) proxy_url_input: Entity<InputState>,
    /// 代理提示浮层显示状态
    pub(crate) proxy_tips_hovered: bool,
    pub(crate) proxy_tips_x: Option<f32>,
    pub(crate) proxy_tips_y: Option<f32>,
    pub(crate) show_shortcuts_popup: bool,
    /// 代码生成对话框
    pub(crate) code_gen_dialog_state: Arc<Mutex<crate::ui::dialogs::CodeGenDialogState>>,
    /// 根元素 FocusHandle（用于确保快捷键始终生效）
    pub(crate) root_focus_handle: gpui::FocusHandle,
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

impl gpui::Focusable for MainView {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.root_focus_handle.clone()
    }
}

/// 高亮文本渲染。
///
/// 片段是缓存的（`build()` 里算过一次），这里每帧只做两件事：
/// `SharedString::clone()`（引用计数 +1）和建 div。
/// 旧实现是「每个字符一个 div + 一次 `to_string()`」，
/// 一段几十万字符的响应每帧要产生几十万次分配。
fn render_highlighted_lines(lines: &[crate::ui::response_highlight::Line]) -> Vec<AnyElement> {
    lines
        .iter()
        .map(|line| {
            div()
                .flex()
                .flex_row()
                .children(line.iter().map(|run| {
                    div()
                        .flex_none()
                        .text_color(run.color)
                        .child(run.text.clone())
                        .into_any_element()
                }))
                .into_any_element()
        })
        .collect()
}

/// 根据 Content-Type 智能选择预览渲染方式
fn content_type_to_image_format(ct: &str) -> Option<ImageFormat> {
    match ct {
        "image/png" => Some(ImageFormat::Png),
        "image/jpeg" | "image/jpg" => Some(ImageFormat::Jpeg),
        "image/gif" => Some(ImageFormat::Gif),
        "image/webp" => Some(ImageFormat::Webp),
        "image/bmp" => Some(ImageFormat::Bmp),
        "image/svg+xml" => Some(ImageFormat::Svg),
        "image/tiff" => Some(ImageFormat::Tiff),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some(ImageFormat::Ico),
        _ => None,
    }
}

fn decode_image_bytes(raw: &[u8], format: ImageFormat) -> Option<Arc<RenderImage>> {
    let img_format = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Gif => image::ImageFormat::Gif,
        ImageFormat::Webp => image::ImageFormat::WebP,
        ImageFormat::Bmp => image::ImageFormat::Bmp,
        ImageFormat::Tiff => image::ImageFormat::Tiff,
        ImageFormat::Ico => image::ImageFormat::Ico,
        ImageFormat::Svg | ImageFormat::Pnm => return None,
    };
    let mut buffer = image::load_from_memory_with_format(raw, img_format).ok()?.into_rgba8();
    for pixel in buffer.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let frame = image::Frame::new(buffer);
    let render = RenderImage::new(smallvec::smallvec![frame]);
    Some(Arc::new(render))
}

fn render_preview_body(
    body: &str,
    content_type: Option<&str>,
    theme: &Theme,
    t: &dyn Fn(&str) -> SharedString,
    raw_body: Option<&[u8]>,
    // 高亮结果由 MainView 缓存后传进来；None 只会在缓存缺失时出现
    highlight: Option<&crate::ui::response_highlight::HighlightedBody>,
) -> AnyElement {
    let ct = content_type.unwrap_or("").to_lowercase();

    // 光栅图片 — 有原始字节时预解码渲染+滚动，否则显示占位
    if ct.starts_with("image/") && ct != "image/svg+xml" {
        if let (Some(raw), Some(format)) = (raw_body, content_type_to_image_format(&ct)) {
            // 图片渲染已在 BodyViewMode::Preview 分支处理，此处仅作 fallback
            if raw_body.is_some() {
                return div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child("Image response detected")
                    .into_any_element();
            }
        }
        return div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .children([
                div()
                    .text_color(theme.muted_foreground)
                    .child("Image response detected"),
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "Content-Type: {}",
                        content_type.unwrap_or("unknown")
                    )),
            ])
            .into_any_element();
    }

    // SVG — 作为 XML 源码显示
    if ct.starts_with("image/svg+xml") {
        return div()
            .h_full()
            .w_full()
            .overflow_y_scrollbar()
            .bg(theme.code_background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .p_3()
            .text_sm()
            .child(body.to_string())
            .into_any_element();
    }

    // HTML — "在浏览器中打开"按钮 + 下方显示源码
    if ct.contains("html") {
        let body_owned = body.to_string();
        return div()
            .h_full()
            .flex_col()
            .overflow_hidden()
            .children([
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .p_2()
                    .children([
                        div()
                            .text_color(theme.muted_foreground)
                            .text_sm()
                            .child("HTML Response"),
                        div()
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(theme.accent)
                            .text_color(theme.accent_foreground)
                            .text_sm()
                            .child(t("preview.open_in_browser"))
                            .on_mouse_down(MouseButton::Left, {
                                let body = body_owned.clone();
                                move |_event, _window, _cx| {
                                    let tmp_path = std::env::temp_dir().join(format!(
                                        "apipost-preview-{}.html",
                                        uuid::Uuid::new_v4()
                                    ));
                                    if let Err(e) = std::fs::write(&tmp_path, &body) {
                                        log::error!("Failed to write temp HTML file: {}", e);
                                        return;
                                    }
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg(tmp_path.to_string_lossy().to_string())
                                        .spawn();
                                }
                            }),
                    ]),
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .px_2()
                    .child(t("preview.view_source")),
                div()
                    .w_full()
                    .flex_1()
                    .flex_col()
                    .bg(theme.code_background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .overflow_y_scrollbar()
                            .p_3()
                            .text_xs()
                            .text_color(theme.foreground)
                            .child(body.to_string()),
                    ),
            ])
            .into_any_element();
    }

    // JSON — 纯文本展示（不格式化，保持原始宽度，自然换行）
    if ct.contains("json") {
        return div()
            .h_full()
            .w_full()
            .overflow_y_scrollbar()
            .bg(theme.code_background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .p_3()
            .text_sm()
            .text_color(theme.foreground)
            .child(body.to_string())
            .into_any_element();
    }

    // PDF — 在外部程序中打开
    if ct.starts_with("application/pdf") {
        let pdf_bytes = raw_body.map(|r| r.to_vec());
        return div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .children([
                div()
                    .text_color(theme.muted_foreground)
                    .child("PDF Response"),
                div()
                    .cursor_pointer()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(theme.accent)
                    .text_color(theme.accent_foreground)
                    .text_sm()
                    .child(t("preview.open_external"))
                    .on_mouse_down(MouseButton::Left, {
                        let pdf_bytes = pdf_bytes.clone();
                        move |_event, _window, _cx| {
                            let tmp_path = std::env::temp_dir()
                                .join(format!("apipost-preview-{}.pdf", uuid::Uuid::new_v4()));
                            let data = pdf_bytes.as_deref().unwrap_or(b"");
                            if let Err(e) = std::fs::write(&tmp_path, data) {
                                log::error!("Failed to write temp PDF file: {}", e);
                                return;
                            }
                            let _ = std::process::Command::new("xdg-open")
                                .arg(tmp_path.to_string_lossy().to_string())
                                .spawn();
                        }
                    }),
            ])
            .into_any_element();
    }

    // 文本类型及其他 —— 解析 / 美化 / 分词的结果由 MainView 缓存，这里只负责摆放
    let fallback;
    let highlighted = match highlight {
        Some(cached) => cached,
        None => {
            // 兜底：缓存还没建立时现算一次（正常每帧都命中缓存）
            fallback = crate::ui::response_highlight::build(body, content_type, theme);
            &fallback
        }
    };

    div()
        .h_full()
        .w_full()
        .overflow_y_scrollbar()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .p_3()
        .text_sm()
        .child(match highlighted.plain.as_ref() {
            // 非 JSON：直接复用响应体的 Arc<str>（零拷贝），保留原来的自动换行
            Some(text) => div()
                .text_color(theme.foreground)
                .child(SharedString::from(Arc::clone(text))),
            None => div().flex_col().children(render_highlighted_lines(&highlighted.lines)),
        })
        .into_any_element()
}

impl MainView {
    /// 获取翻译文本（从 Arc 缓存读取，无 Mutex 锁争用）
    ///
    /// 返回 `Arc<str>`：命中时只是引用计数 +1，不再像以前那样
    /// 每帧为界面上每一条文案都复制一个 String（每帧上百次堆分配）。
    pub(crate) fn t(&self, key: &str) -> SharedString {
        match self.translations.get(key) {
            // 命中：Arc -> SharedString 是 O(1)，不复制字符串
            Some(value) => SharedString::from(Arc::clone(value)),
            // 只有翻译缺失才会走到这里
            None => SharedString::from(key),
        }
    }

    /// 按需重建响应体高亮缓存。
    ///
    /// 放在 render 开头：这里是唯一能同时拿到 `&mut self` 和「每帧都会执行」
    /// 两个条件的地方。命中缓存时只是指针比较 + 一次主题比较，几乎免费；
    /// 真正昂贵的 parse / 美化 / 分词只在响应到达或主题切换时跑一次。
    fn ensure_response_highlight(&mut self) {
        let Some(resp) = self.response.as_ref() else {
            self.response_highlight = None;
            return;
        };

        if let Some(cache) = self.response_highlight.as_ref() {
            if cache.is_valid_for(&resp.body, &self.cached_theme) {
                return;
            }
        }

        // 走这里说明要重算：Content-Type 也一起取出来用（与旧实现同一套判断）
        let content_type = resp.detect_content_type();
        let highlighted = crate::ui::response_highlight::build(
            &resp.body,
            content_type.as_deref(),
            &self.cached_theme,
        );
        let body = Arc::clone(&resp.body);
        let theme = self.cached_theme.clone();
        self.response_highlight =
            Some(crate::ui::response_highlight::Cache::new(body, theme, highlighted));
    }

    /// 语言切换后更新翻译缓存
    fn refresh_translations(&mut self) {
        self.translations = self.app_state.lock().unwrap().i18n.translations_arc();
    }

    /// 创建构建器标签页按钮（带i18n支持）
    fn builder_tab_button(
        &self,
        cx: &Context<Self>,
        label_key: &str,
        tab: BuilderTab,
        current_tab: BuilderTab,
        id: impl Into<ElementId>,
    ) -> impl IntoElement {
        let theme = self.cached_theme.clone();
        let is_active = current_tab == tab;
        // 与响应区标签页共用同一套样式（高度、下划线、hover），避免左右两栏观感不一致
        crate::ui::components::pane_tab(id, self.t(label_key), is_active, &theme).on_click(
            cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_builder_tab(tab, cx);
            }),
        )
    }

    /// 创建响应标签页按钮（带i18n支持）
    fn response_tab_button(
        &self,
        cx: &Context<Self>,
        label_key: &str,
        tab: ResponseTab,
        current_tab: ResponseTab,
        id: impl Into<ElementId>,
    ) -> impl IntoElement {
        let theme = self.cached_theme.clone();
        let is_active = current_tab == tab;
        crate::ui::components::pane_tab(id, self.t(label_key), is_active, &theme).on_click(
            cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                this.set_response_tab(tab, cx);
            }),
        )
    }

    /// 创建新的主视图
    pub fn new(
        app_state: Arc<std::sync::Mutex<crate::app::AppState>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // 缓存翻译字典（Arc 共享，避免每次翻译都获取 Mutex 锁）
        let translations = app_state.lock().unwrap().i18n.translations_arc();
        // 缓存主题（避免每个面板渲染时重复调用 Theme::from_str）
        let cached_theme = Theme::from_str(&app_state.lock().unwrap().theme_name);
        // 响应体高亮缓存：首帧渲染时按需建立
        let response_highlight = None;

        // 加载历史记录
        let history = app_state
            .lock()
            .unwrap()
            .db
            .get_history(50, 0)
            .unwrap_or_default();

        // 创建URL输入状态
        let url_input = cx.new(|cx| {
            let guard = app_state.lock().unwrap();
            let placeholder = guard.i18n.get("request.url.placeholder").to_string();
            drop(guard);
            InputState::new(window, cx).placeholder(placeholder)
        });

        // 设置URL输入变化订阅（URL变化时自动解析到params）
        let url_input_clone = url_input.clone();
        let _url_change_sub = cx.subscribe_in(
            &url_input,
            window,
            move |this, _state, event, _window, cx| {
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
                        // URL变更后保存工作区
                        this.save_workspace(cx);
                    }
                    _ => {}
                }
            },
        );

        // 创建HTTP方法选择状态
        let methods: Vec<crate::ui::components::MethodItem> =
            ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                .iter()
                .map(|m| crate::ui::components::MethodItem::new(*m))
                .collect();
        let method_select =
            cx.new(|cx| SelectState::new(methods, Some(IndexPath::default()), window, cx));

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
        let body_state = BodyState::new(
            raw_content,
            raw_content_xml,
            raw_content_text,
            raw_content_html,
        );

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

        // 创建响应体 Pretty 输入状态
        let response_pretty_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
                .line_number(true)
                .soft_wrap(true)
                .default_value("")
        });

        // 响应头输入状态（动态创建，每个头一个键值对）
        let response_header_inputs: Vec<(Entity<InputState>, Entity<InputState>)> = Vec::new();

        // 创建响应体Raw格式选择器
        let response_raw_format_select = BodyState::create_raw_format_select(window, cx);

        // 获取默认标签页名称
        let default_tab_name = app_state.lock().unwrap().i18n.get("sidebar.new_request").to_string();
        let saved_requests = app_state
            .lock()
            .unwrap()
            .db
            .get_saved_requests()
            .unwrap_or_default();
        let folders = app_state
            .lock()
            .unwrap()
            .db
            .get_folders()
            .unwrap_or_default();
        let environments = app_state
            .lock()
            .unwrap()
            .db
            .get_environments()
            .unwrap_or_default();
        let active_environment_name = environments
            .iter()
            .find(|e| e.is_active)
            .map(|e| e.name.clone());
        let env_dialog_state = Arc::new(Mutex::new(EnvDialogState::new(window, cx)));
        let folder_dialog_state = Arc::new(Mutex::new(FolderDialogState::new(window, cx)));
        let move_dialog_state = Arc::new(Mutex::new(MoveDialogState::new()));
        let save_request_dialog = Arc::new(Mutex::new(SaveRequestDialog::new(window, cx)));
        let code_gen_dialog_state = Arc::new(Mutex::new(
            crate::ui::dialogs::CodeGenDialogState::new(window, cx),
        ));

        let proxy_url = app_state.lock().unwrap().config.proxy.url.clone();
        let proxy_url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&proxy_url)
        });

        Self {
            app_state,
            translations,
            cached_theme,
            response_highlight,
            method: "GET".to_string(),
            url: String::new(),
            request_tabs: vec![RequestTab {
                id: 1,
                method: "GET".to_string(),
                url: String::new(),
                name: default_tab_name,
                tab_state: TabState::default(),
            }].into(),
            active_tab: 0,
            tabs_scroll: gpui::ScrollHandle::new(),
            tabs_reveal_pending: true,
            tabs_viewport: 0.0,
            response: None,
            response_tab: ResponseTab::Body,
            body_view_mode: BodyViewMode::Pretty,
            is_loading: false,
            loading_frame: 0,
            error_message: None,
            sidebar_collapsed: false,
            sidebar_tab: SidebarTab::Collections,
            show_settings_popover: false,
            update_status: crate::app::updater::UpdateStatus::Idle,
            update_checked_at: None,
            context_menu_target: None,
            hovered_item_name: None,
            hovered_item_y: None,
            hovered_item_x: None,
            context_menu_pos: None,
            expanded_folders: HashSet::new(),
            collection_items: Arc::new(crate::ui::sidebar::build_collection_tree(
                &folders,
                &saved_requests,
                &HashSet::new(),
            )),
            needs_collections_refresh: false,
            needs_drop_refresh: Arc::new(AtomicBool::new(false)),
            history: Arc::new(history),
            saved_requests: Arc::new(saved_requests),
            folders: Arc::new(folders),
            environments: Arc::new(environments),
            active_environment_name,
            env_dialog_state,
            folder_dialog_state,
            move_dialog_state,
            url_input,
            _url_change_sub,
            _param_input_subs: Vec::new(),
            _header_subs: Vec::new(),
            _form_data_type_subs: Vec::new(),
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
            response_pretty_input,
            response_header_inputs,
            response_raw_format: RawFormat::Json,
            response_raw_format_select,
            response_soft_wrap: false,
            next_tab_id: 2,
            splitter_dragging: false,
            splitter_start_y: 0.0,
            last_splitter_update_y: 0.0,
            request_builder_height: 400.0,
            response_editor_height: 400.0,
            response_editor_dragging: false,
            response_editor_start_y: 0.0,
            save_request_dialog,
            last_workspace_save: std::time::Instant::now(),
            auto_save_task: None,
            proxy_url_input,
            proxy_tips_hovered: false,
            proxy_tips_x: None,
            proxy_tips_y: None,
            show_shortcuts_popup: false,
            code_gen_dialog_state,
            root_focus_handle: cx.focus_handle(),
        }
    }

    /// 启动/停止自动保存后台任务
    pub fn update_auto_save_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let auto_save_enabled = self.app_state.lock().unwrap().config.general.auto_save;
        if auto_save_enabled && self.auto_save_task.is_none() {
            log::info!("启动自动保存 (30s 间隔)");
            self.auto_save_task = Some(cx.spawn_in(window, async move |this: WeakEntity<MainView>, cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(30))
                        .await;
                    let should_continue = this.update(cx, |this, cx| {
                        let auto_save = this.app_state.lock().unwrap().config.general.auto_save;
                        if auto_save {
                            this.save_workspace(cx);
                            true
                        } else {
                            log::info!("自动保存已关闭，停止后台任务");
                            false
                        }
                    });
                    match should_continue {
                        Ok(true) => {}
                        _ => break,
                    }
                }
            }));
        } else if !auto_save_enabled {
            self.auto_save_task = None;
        }
    }

    /// 发送HTTP请求
    pub fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }

        self.is_loading = true;
        self.loading_frame = 0;
        self.error_message = None;
        cx.notify();

        // 同步 params 到 URL（必须同步完成）
        self.sync_params_to_url(window, cx);

        // 克隆请求所需数据，其余全部移入异步任务避免阻塞 rendering
        let method = self.method.clone();
        let url = self.url.clone();
        let auth_headers = self.auth_state.to_headers(cx);
        let user_headers: Vec<(String, String)> = self
            .headers
            .iter()
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
        let mut all_headers = auth_headers;
        all_headers.extend(user_headers);
        let content_type = self.body_state.content_type();
        if let Some(ref ct) = content_type {
            if !all_headers
                .iter()
                .any(|(k, _)| k.to_lowercase() == "content-type")
            {
                all_headers.push(("Content-Type".to_string(), ct.clone()));
            }
        }
        let headers_text_for_history: String = all_headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        let body = self.body_state.to_body(cx);
        let resolved_url = self
            .app_state
            .lock()
            .unwrap()
            .env_manager
            .replace_variables(&self.url)
            .into_owned();
        let app_state = self.app_state.clone();

        // 请求构造与发送放到下一帧，确保当前帧先渲染 loading overlay
        cx.on_next_frame(window, move |this, window, cx| {
            // 构建完整 URL
            let url_with_scheme =
                if !resolved_url.starts_with("http://") && !resolved_url.starts_with("https://") {
                    format!("http://{}", resolved_url)
                } else {
                    resolved_url
                };
            let base_url = if let Some(query_start) = url_with_scheme.find('?') {
                url_with_scheme[..query_start].to_string()
            } else {
                url_with_scheme
            };
            let params: Vec<(String, String, bool)> = this
                .params
                .iter()
                .map(|p| {
                    let key = p.key.read(cx).value().to_string();
                    let value = p.value.read(cx).value().to_string();
                    (key, value, p.enabled)
                })
                .collect();
            let enabled_params: Vec<&(String, String, bool)> =
                params.iter().filter(|p| p.2 && !p.0.is_empty()).collect();
            let mut full_url = if enabled_params.is_empty() {
                base_url
            } else {
                let query_string: String = enabled_params
                    .iter()
                    .map(|(key, value, _)| {
                        format!(
                            "{}={}",
                            urlencoding::encode(key),
                            urlencoding::encode(value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                format!("{}?{}", base_url, query_string)
            };
            if let Some((key, value)) = this.auth_state.to_query_params(cx) {
                let encoded_key = urlencoding::encode(&key);
                let encoded_value = urlencoding::encode(&value);
                if full_url.contains('?') {
                    full_url = format!("{}&{}={}", full_url, encoded_key, encoded_value);
                } else {
                    full_url = format!("{}?{}={}", full_url, encoded_key, encoded_value);
                }
            }
            let body = this.body_state.to_body(cx);
            let text_fields = this.body_state.get_form_data_text_fields(cx);
            let mut file_fields_raw = this.body_state.get_form_data_file_fields(cx);
            // Binary 模式下添加单独的文件
            if this.body_state.body_type == BodyType::Binary {
                if let Some(ref path) = this.body_state.binary_file_path {
                    file_fields_raw.push((
                        "file".to_string(),
                        path.clone(),
                        "application/octet-stream".to_string(),
                    ));
                }
            }
            let file_fields: Vec<crate::http::FileField> = file_fields_raw
                .into_iter()
                .map(
                    |(field_name, file_path, content_type)| crate::http::FileField {
                        field_name,
                        file_path,
                        content_type,
                    },
                )
                .collect();
            let body_for_history = body.clone();
            // 为 spawn_in 异步闭包克隆需要的值（其余直接 move 到 request 中）
            let method_for_history = method.clone();
            let url_for_history = url.clone();
            let app_state = app_state.clone();

            let request = HttpRequest {
                method, // move（on_next_frame 捕获的 method）
                url: full_url,
                headers: all_headers, // move（不再 clone）
                body,
                content_type,
                text_fields,
                file_fields,
            };

            let (tx, rx) = std::sync::mpsc::channel();
            let http_client = {
                let state = app_state.lock().unwrap();
                state.http_client.clone()
            }; // 立即释放 app_state 锁，避免阻塞 UI 渲染
            let rt_handle = {
                let state = app_state.lock().unwrap();
                state.rt_handle.clone()
            };
            std::thread::spawn(move || {
                let result = rt_handle.block_on(http_client.send_request(&request));
                let _ = tx.send(result);
            });

            cx.spawn_in(window, async move |this: WeakEntity<MainView>, cx| {
                let result = loop {
                    match rx.try_recv() {
                        Ok(r) => break r,
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(100))
                                .await;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            break Err(anyhow::anyhow!("请求线程异常退出"));
                        }
                    }
                };
                this.update_in(cx, |this, window, cx| {
                    match result {
                        Ok(response) => {
                            let history_entry = CreateHistoryEntry {
                                method: method_for_history,
                                url: url_for_history,
                                headers: Some(headers_text_for_history),
                                body: body_for_history,
                                response_status: Some(response.status as i32),
                                response_headers: Some(
                                    serde_json::to_string(&response.headers).unwrap_or_default(),
                                ),
                                response_body: Some(response.body.to_string()),
                                response_time_ms: Some(response.time_ms),
                                response_size: Some(response.size_bytes),
                            };
                            if let Err(e) = this
                                .app_state
                                .lock()
                                .unwrap()
                                .db
                                .add_history(&history_entry.into_history_entry())
                            {
                                log::error!("保存历史记录失败: {}", e);
                            }
                            // 包一次 Arc 再共享：MainView / TabState / 工作区快照都拿引用计数，
                            // 后面还要读 response 的 body/headers，所以先包再用
                            let response = Arc::new(response);
                            this.response = Some(response.clone());
                            let content_type = response.detect_content_type();
                            this.response_raw_format =
                                RawFormat::detect(content_type.as_deref(), response.body.as_ref());
                            let body_ref = response.body.as_ref();
                            this.response_input.update(cx, |state, cx| {
                                state.set_value(body_ref, window, cx);
                            });
                            this.response_xml_input.update(cx, |state, cx| {
                                state.set_value(body_ref, window, cx);
                            });
                            this.response_text_input.update(cx, |state, cx| {
                                state.set_value(body_ref, window, cx);
                            });
                            this.response_html_input.update(cx, |state, cx| {
                                state.set_value(body_ref, window, cx);
                            });
                            this.update_pretty_editor(window, cx);
                            this.rebuild_response_header_inputs(&response.headers, window, cx);
                            if let Ok(hist) = this.app_state.lock().unwrap().db.get_history(50, 0) {
                                this.history = Arc::new(hist);
                            }
                            // 响应成功后立即保存工作区状态
                            this.save_workspace(cx);
                        }
                        Err(e) => {
                            log::error!("请求失败: {}", e);
                            this.error_message = Some(format!("请求失败 -> {}", e));
                            this.save_workspace(cx);
                        }
                    }
                    this.is_loading = false;
                    this.loading_frame = 0;
                    cx.notify();
                })
                .ok();
            })
            .detach();
        });
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
        log::debug!("set_sidebar_tab: 切换到标签页 {:?}", tab);
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
        self.last_splitter_update_y = start_y;
    }

    /// 更新splitter位置（拖拽中）
    pub fn update_splitter_drag(&mut self, current_y: f32) -> bool {
        if self.splitter_dragging {
            let delta_y = current_y - self.splitter_start_y;
            self.request_builder_height = (self.request_builder_height + delta_y).max(100.0);
            self.splitter_start_y = current_y;
            // 节流：每 6px 移动才通知一次渲染，减少重绘频率
            if (current_y - self.last_splitter_update_y).abs() > 6.0 {
                self.last_splitter_update_y = current_y;
                return true;
            }
        }
        false
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
        self.params.push(ParamEntry {
            key,
            value,
            enabled: true,
        });
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
    fn rebuild_response_header_inputs(
        &mut self,
        headers: &std::collections::HashMap<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.response_header_inputs.clear();
        for (k, v) in headers.iter() {
            let key_input = cx.new(|cx| InputState::new(window, cx).default_value(k));
            let val_input = cx.new(|cx| InputState::new(window, cx).default_value(v));
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
        // 重复点同一个类型直接返回：否则会把已填的 token / 账号密码清空
        if self.get_auth_type() == auth_type {
            return;
        }
        self.auth_state = match auth_type {
            AuthType::NoAuth => AuthState::NoAuth,
            AuthType::BearerToken => {
                let token = cx.new(|cx| InputState::new(window, cx).default_value(""));
                AuthState::Bearer(crate::ui::BearerTokenAuthData { token })
            }
            AuthType::BasicAuth => {
                let username = cx.new(|cx| InputState::new(window, cx).default_value(""));
                // 密码做遮蔽显示
                let password = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value("")
                        .masked(true)
                });
                AuthState::Basic(crate::ui::BasicAuthData { username, password })
            }
            AuthType::ApiKey => {
                let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
                let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
                let locations = ApiKeyLocation::all();
                let location = cx
                    .new(|cx| SelectState::new(locations, Some(IndexPath::default()), window, cx));
                AuthState::ApiKey(crate::ui::ApiKeyAuthData {
                    key,
                    value,
                    location,
                    location_value: ApiKeyLocation::Header,
                })
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

    /// 设置 API Key 认证的位置（Header / Query）
    pub fn set_api_key_location(&mut self, location: ApiKeyLocation, cx: &mut Context<Self>) {
        if let AuthState::ApiKey(auth) = &mut self.auth_state {
            if auth.location_value != location {
                auth.location_value = location;
                cx.notify();
            }
        }
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
    pub fn set_response_raw_format(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.response_raw_format = RawFormat::from_index(index);
        self.update_pretty_editor(window, cx);
        cx.notify();
    }

    pub fn update_pretty_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        let formatted = if let Some(resp) = &self.response {
            let body = resp.body.as_ref();
            match self.response_raw_format {
                RawFormat::Json => serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| body.to_string()),
                _ => body.to_string(),
            }
        } else {
            String::new()
        };
        self.response_pretty_input.update(cx, |state, cx| {
            state.set_value(&formatted, window, cx);
        });
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
        self.rebuild_form_data_type_subscriptions(window, cx);
    }

    /// 删除 form-data 条目
    pub fn remove_form_data_entry(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.body_state.remove_form_data_entry(index);
        self.rebuild_form_data_type_subscriptions(window, cx);
    }

    /// 切换 form-data 条目启用状态
    pub fn toggle_form_data_entry(&mut self, index: usize, cx: &mut Context<Self>) {
        self.body_state.toggle_form_data_entry(index);
        cx.notify();
    }

    /// 设置 form-data 条目类型
    pub fn set_form_data_param_type(
        &mut self,
        index: usize,
        param_type: crate::ui::FormDataParamType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.body_state
            .set_form_data_param_type(index, param_type, window, cx);
        cx.notify();
    }

    /// 为 form-data File 类型选择文件
    pub fn pick_file_for_binary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use rfd::FileDialog;
        if let Some(file_path) = FileDialog::new().pick_file() {
            self.body_state.binary_file_path = Some(file_path.to_string_lossy().to_string());
            cx.notify();
        }
    }

    pub fn pick_file_for_form_data(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use rfd::FileDialog;

        if index >= self.body_state.form_data.len() {
            return;
        }

        let entry = &self.body_state.form_data[index];
        if entry.param_type != crate::ui::FormDataParamType::File {
            return;
        }

        // 打开文件选择对话框
        if let Some(file_path) = FileDialog::new().pick_file() {
            let path_str = file_path.to_string_lossy().to_string();
            self.body_state
                .update_form_data_file_path(index, &path_str, window, cx);
            cx.notify();
        }
    }

    /// 设置 url-encoded 条目类型
    pub fn set_urlencoded_param_type(
        &mut self,
        index: usize,
        param_type: crate::ui::FormDataParamType,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.body_state
            .set_urlencoded_param_type(index, param_type, window, cx);
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
    fn parse_url_to_params_internal(
        &mut self,
        url: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
                    let decoded_key = urlencoding::decode(key)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| key.to_string());
                    let decoded_value = urlencoding::decode(value)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| value.to_string());

                    let key_entity =
                        cx.new(|cx| InputState::new(window, cx).default_value(&decoded_key));
                    let value_entity =
                        cx.new(|cx| InputState::new(window, cx).default_value(&decoded_value));

                    self.params.push(ParamEntry {
                        key: key_entity,
                        value: value_entity,
                        enabled: true,
                    });
                } else {
                    let decoded_key = urlencoding::decode(pair)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| pair.to_string());
                    let key_entity =
                        cx.new(|cx| InputState::new(window, cx).default_value(&decoded_key));
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
        let params: Vec<(String, String, bool)> = self
            .params
            .iter()
            .map(|p| {
                let key = p.key.read(cx).value().to_string();
                let value = p.value.read(cx).value().to_string();
                (key, value, p.enabled)
            })
            .collect();

        let enabled_params: Vec<&(String, String, bool)> =
            params.iter().filter(|p| p.2 && !p.0.is_empty()).collect();

        let new_url = if enabled_params.is_empty() {
            base_url
        } else {
            let query_string: String = enabled_params
                .iter()
                .map(|(key, value, _)| {
                    format!(
                        "{}={}",
                        urlencoding::encode(key),
                        urlencoding::encode(value)
                    )
                })
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
    pub(crate) fn rebuild_param_subscriptions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._param_input_subs.clear();
        for param in &self.params {
            let key = param.key.clone();
            let value = param.value.clone();
            let key_sub = cx.subscribe_in(&key, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_params_to_url(_window, cx);
                }
            });
            let value_sub =
                cx.subscribe_in(&value, window, move |this, _state, event, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.sync_params_to_url(_window, cx);
                    }
                });
            self._param_input_subs.push(key_sub);
            self._param_input_subs.push(value_sub);
        }
    }

    /// 重建所有 header key/value 的输入变化订阅，检测 Content-Type 自动切换 Body 类型
    pub(crate) fn rebuild_header_subscriptions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._header_subs.clear();
        for header in &self.headers {
            let key = header.key.clone();
            let value = header.value.clone();
            let key_sub = cx.subscribe_in(&key, window, move |this, _state, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.auto_detect_body_type_from_headers(_window, cx);
                }
            });
            let value_sub =
                cx.subscribe_in(&value, window, move |this, _state, event, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.auto_detect_body_type_from_headers(_window, cx);
                    }
                });
            self._header_subs.push(key_sub);
            self._header_subs.push(value_sub);
        }
    }

    /// 重建所有 form-data 条目的类型选择订阅
    pub(crate) fn rebuild_form_data_type_subscriptions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._form_data_type_subs.clear();
        for (idx, entry) in self.body_state.form_data.iter().enumerate() {
            let type_select = entry.type_select.clone();
            let sub = cx.subscribe_in(
                &type_select,
                window,
                move |this, _state, event, _window, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        let param_type = match value.as_ref() {
                            "Text" => FormDataParamType::Text,
                            "Boolean" => FormDataParamType::Boolean,
                            "Number" => FormDataParamType::Number,
                            "File" => FormDataParamType::File,
                            "Array" => FormDataParamType::Array,
                            _ => return,
                        };
                        this.set_form_data_param_type(idx, param_type, _window, cx);
                    }
                },
            );
            self._form_data_type_subs.push(sub);
        }
    }

    /// 根据 Content-Type header 自动切换 Body 类型
    pub(crate) fn auto_detect_body_type_from_headers(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            if ct_lower.contains("json")
                || ct_lower.contains("xml")
                || ct_lower.contains("html")
                || ct_lower.contains("text")
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
    pub fn import_curl(
        &mut self,
        curl_command: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
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
            (
                &request.url[..query_pos],
                Some(&request.url[query_pos + 1..]),
            )
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
        self.save_workspace(cx);
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
        // 先构造再 push：push 的实参里会借用 self（self.t(...)），
        // 与 Arc::make_mut(&mut self.request_tabs) 的可变借用冲突
        let new_tab_entry = RequestTab {
            id,
            method: "GET".to_string(),
            url: String::new(),
            name: self.t("sidebar.new_request").to_string(),
            tab_state: TabState::default(),
        };
        Arc::make_mut(&mut self.request_tabs).push(new_tab_entry);
        let new_idx = self.request_tabs.len() - 1;
        self.active_tab = new_idx;
        // 让新标签滚动到可见区域
        self.reveal_tab(new_idx, self.tabs_viewport);

        // 加载默认 TabState，重置所有 UI 组件（body/auth/settings/scripts 等）
        let new_tab = self.request_tabs[new_idx].clone();
        self.load_tab_meta(&new_tab, window, cx);

        self.save_workspace(cx);
        cx.notify();
    }

    /// 保存所有 tab 状态到数据库（节流：500ms 内重复调用跳过，避免快速输入时频繁写入）
    pub fn save_workspace(&mut self, cx: &mut Context<Self>) {
        self.save_current_tab_meta(cx);

        // 节流：500ms 内的重复调用跳过
        let now = std::time::Instant::now();
        if now.duration_since(self.last_workspace_save) < std::time::Duration::from_millis(500) {
            return;
        }
        self.last_workspace_save = now;

        let tabs: Vec<&TabState> = self.request_tabs.iter().map(|t| &t.tab_state).collect();
        if let Ok(json) = serde_json::to_string(&tabs) {
            let app = self.app_state.lock().unwrap();
            if let Err(e) = app.db.save_workspace_state(&json, self.active_tab) {
                log::error!("保存工作区失败: {}", e);
            } else {
                log::info!("工作区已保存 ({} tabs)", self.request_tabs.len());
            }
        }
    }

    /// 从数据库恢复上次退出时的 tab 状态
    pub fn load_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let workspace = {
            let app = self.app_state.lock().unwrap();
            app.db.load_workspace_state()
        };
        if let Ok((json, active_tab)) = workspace {
            if let Ok(tabs) = serde_json::from_str::<Vec<TabState>>(&json) {
                if !tabs.is_empty() {
                    Arc::make_mut(&mut self.request_tabs).clear();
                    for (i, ts) in tabs.into_iter().enumerate() {
                        let method = ts.method.clone();
                        let url = ts.url.clone();
                        let name = if ts.name.is_empty() {
                            self.t("sidebar.new_request").to_string()
                        } else {
                            ts.name.clone()
                        };
                        Arc::make_mut(&mut self.request_tabs).push(RequestTab {
                            id: i,
                            method,
                            url,
                            name,
                            tab_state: ts,
                        });
                    }
                    self.next_tab_id = self.request_tabs.len();
                    let idx = active_tab.min(self.request_tabs.len() - 1);
                    self.active_tab = idx;
                    // 布局完成后把当前标签滚动到可见区域（标签较多时）
                    self.tabs_reveal_pending = true;
                    let tab = self.request_tabs[idx].clone();
                    self.load_tab_meta(&tab, window, cx);
                    log::info!("工作区已恢复 ({} tabs)", self.request_tabs.len());
                    return;
                }
            }
        }
    }


    /// 把指定标签滚动到可见范围内。
    ///
    /// 标签是定宽（见 REQUEST_TAB_WIDTH），所以第 idx 个标签在标签条内的位置可以直接算出来；
    /// 这里直接设置滚动偏移，避免依赖下一帧才生效的 scroll_to_item。
    /// viewport 为标签条可视宽度，返回 false 表示宽度还未知（尚未布局）。
    fn reveal_tab(&self, idx: usize, viewport: f32) -> bool {
        /// 标签占位宽度：标签宽度 + 间距
        const TAB_PITCH: f32 = REQUEST_TAB_WIDTH + REQUEST_TAB_GAP;

        if viewport <= 0.0 {
            return false;
        }
        let tab_left = TAB_PITCH * idx as f32;
        // offset 为正数，表示内容向左滚动的距离
        let mut offset = -self.tabs_scroll.offset().x.as_f32();
        if tab_left < offset {
            // 标签在可见区域左侧
            offset = tab_left;
        } else if tab_left + REQUEST_TAB_WIDTH > offset + viewport {
            // 标签在可见区域右侧
            offset = tab_left + REQUEST_TAB_WIDTH - viewport;
        }
        // 这里不按 max_offset 夹紧：新增标签时布局还没更新，max_offset 仍是旧值，
        // 夹紧会导致新标签滚不出来；越界部分由 gpui 在 prepaint 时按最新布局夹紧
        self.tabs_scroll
            .set_offset(point(px(-offset.max(0.0)), px(0.0)));
        true
    }

    /// 按整个标签为单位左右滚动标签条（供标签条两侧的左右按钮使用）。
    /// 以标签宽度为步进，滚动后标签边界仍与可视区左边缘对齐。
    fn scroll_tabs_by(&self, delta_tabs: i32) {
        /// 标签占位宽度：标签宽度 + 间距
        const TAB_PITCH: f32 = REQUEST_TAB_WIDTH + REQUEST_TAB_GAP;

        let content = TAB_PITCH * self.request_tabs.len() as f32 - REQUEST_TAB_GAP;
        let max = (content - self.tabs_viewport).max(0.0);
        let current = -self.tabs_scroll.offset().x.as_f32();
        let next = ((current / TAB_PITCH).round() + delta_tabs as f32) * TAB_PITCH;
        self.tabs_scroll
            .set_offset(point(px(-next.clamp(0.0, max)), px(0.0)));
    }

    /// 关闭指定标签页（至少保留一个）
    pub fn close_tab(&mut self, tab_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.request_tabs.len() <= 1 {
            return;
        }
        Arc::make_mut(&mut self.request_tabs).remove(tab_idx);

        let new_active = if self.active_tab >= self.request_tabs.len() {
            self.request_tabs.len() - 1
        } else if tab_idx < self.active_tab {
            self.active_tab - 1
        } else {
            self.active_tab.min(self.request_tabs.len() - 1)
        };

        // 直接加载目标标签数据
        self.active_tab = new_active;
        // 关闭标签后确保当前标签仍可见
        self.reveal_tab(new_active, self.tabs_viewport);
        let tab = self.request_tabs[new_active].clone();
        self.load_tab_meta(&tab, window, cx);
        self.save_workspace(cx);
    }

    /// 切换到指定标签页
    pub fn switch_tab(&mut self, tab_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if tab_idx == self.active_tab || tab_idx >= self.request_tabs.len() {
            return;
        }
        self.save_current_tab_meta(cx);
        self.active_tab = tab_idx;
        // 切换到可见区域外的标签时自动滚动出来
        self.reveal_tab(tab_idx, self.tabs_viewport);
        let tab = self.request_tabs[tab_idx].clone();
        self.load_tab_meta(&tab, window, cx);
        self.save_workspace(cx);
    }

    /// 将当前表单的 URL / 方法写回当前标签元数据
    fn save_current_tab_meta(&mut self, cx: &mut Context<Self>) {
        if self.active_tab >= self.request_tabs.len() {
            return;
        }
        let url = self.url_input.read(cx).value().to_string();
        let method = self
            .method_select
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "GET".to_string());
        let short = if url.len() > 30 {
            format!("{}…", &url[..30])
        } else {
            url.clone()
        };
        Arc::make_mut(&mut self.request_tabs)[self.active_tab].url = url.clone();
        Arc::make_mut(&mut self.request_tabs)[self.active_tab].method = method.clone();
        Arc::make_mut(&mut self.request_tabs)[self.active_tab].name = if short.is_empty() {
            self.t("sidebar.new_request").to_string()
        } else {
            short.clone()
        };

        // 提取认证状态数据
        let (
            auth_type_index,
            bearer_token,
            basic_username,
            basic_password,
            api_key_name,
            api_key_value,
            api_key_location,
        ) = match &self.auth_state {
            AuthState::NoAuth => (
                0,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthState::Bearer(a) => (
                1,
                a.token.read(cx).value().to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthState::Basic(a) => (
                2,
                String::new(),
                a.username.read(cx).value().to_string(),
                a.password.read(cx).value().to_string(),
                String::new(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthState::ApiKey(a) => (
                3,
                String::new(),
                String::new(),
                String::new(),
                a.key.read(cx).value().to_string(),
                a.value.read(cx).value().to_string(),
                a.location_value,
            ),
        };

        // 保存完整标签状态
        let state = TabState {
            method: method.clone(),
            url: url.clone(),
            name: short.clone(),
            body_type: self.body_state.body_type,
            raw_format: self.body_state.raw_format,
            raw_json: self.body_state.raw_content.read(cx).value().to_string(),
            raw_xml: self.body_state.raw_content_xml.read(cx).value().to_string(),
            raw_text: self
                .body_state
                .raw_content_text
                .read(cx)
                .value()
                .to_string(),
            raw_html: self
                .body_state
                .raw_content_html
                .read(cx)
                .value()
                .to_string(),
            headers: self
                .headers
                .iter()
                .map(|h| {
                    (
                        h.key.read(cx).value().to_string(),
                        h.value.read(cx).value().to_string(),
                        h.enabled,
                    )
                })
                .collect(),
            params: self
                .params
                .iter()
                .map(|p| {
                    (
                        p.key.read(cx).value().to_string(),
                        p.value.read(cx).value().to_string(),
                        p.enabled,
                    )
                })
                .collect(),
            auth_type_index,
            bearer_token,
            basic_username,
            basic_password,
            api_key_name,
            api_key_value,
            api_key_location,
            settings: self.settings.clone(),
            response: self.response.clone(),
            response_raw_format: self.response_raw_format,
            builder_tab: self.builder_tab,
            timeout_secs: self
                .settings_inputs
                .timeout_input
                .read(cx)
                .value()
                .to_string(),
            retry_count: self
                .settings_inputs
                .retry_input
                .read(cx)
                .value()
                .to_string(),
            form_data: self
                .body_state
                .form_data
                .iter()
                .map(|e| SavedFormDataEntry {
                    key: e.key.read(cx).value().to_string(),
                    value: e.value.get_input_entity().read(cx).value().to_string(),
                    enabled: e.enabled,
                    param_type: e.param_type,
                    is_file: matches!(e.value, FormDataValue::File(_, _)),
                    file_path: match &e.value {
                        FormDataValue::File(_, path) => path.clone(),
                        _ => String::new(),
                    },
                })
                .collect(),
            urlencoded_data: self
                .body_state
                .urlencoded_data
                .iter()
                .map(|e| SavedFormDataEntry {
                    key: e.key.read(cx).value().to_string(),
                    value: e.value.get_input_entity().read(cx).value().to_string(),
                    enabled: e.enabled,
                    param_type: e.param_type,
                    is_file: matches!(e.value, FormDataValue::File(_, _)),
                    file_path: match &e.value {
                        FormDataValue::File(_, path) => path.clone(),
                        _ => String::new(),
                    },
                })
                .collect(),
            pre_request_script: self.script_state.pre_request_script.read(cx).value().to_string(),
            test_script: self.script_state.test_script.read(cx).value().to_string(),
        };
        Arc::make_mut(&mut self.request_tabs)[self.active_tab].tab_state = state;
    }

    /// 将标签元数据加载到表单控件
    fn load_tab_meta(&mut self, tab: &RequestTab, window: &mut Window, cx: &mut Context<Self>) {
        let state = &tab.tab_state;
        self.method = state.method.clone();
        self.url = state.url.clone();
        self.params.clear();
        self.headers.clear();
        self.error_message = None;
        self.response_header_inputs.clear();

        // 先清除订阅，防止 set_value 触发 Change 事件导致错误的同步
        self._param_input_subs.clear();
        self._header_subs.clear();

        let state = &tab.tab_state;

        // 恢复响应
        self.response = state.response.clone();
        self.response_raw_format = state.response_raw_format;

        // URL 和方法
        let url = state.url.clone();
        let method = state.method.clone();
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

        // 恢复 Body 状态
        self.body_state.body_type = state.body_type;
        self.body_state.raw_format = state.raw_format;
        self.body_state.raw_content.update(cx, |s, cx| {
            s.set_value(&state.raw_json, window, cx);
        });
        self.body_state.raw_content_xml.update(cx, |s, cx| {
            s.set_value(&state.raw_xml, window, cx);
        });
        self.body_state.raw_content_text.update(cx, |s, cx| {
            s.set_value(&state.raw_text, window, cx);
        });
        self.body_state.raw_content_html.update(cx, |s, cx| {
            s.set_value(&state.raw_html, window, cx);
        });
        self.body_type_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(state.body_type.to_index())), window, cx);
        });
        self.raw_format_select.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(state.raw_format.to_index())),
                window,
                cx,
            );
        });

        // 恢复 Params
        for (key, value, enabled) in &state.params {
            let key_input = cx.new(|cx| InputState::new(window, cx).default_value(key));
            let value_input = cx.new(|cx| InputState::new(window, cx).default_value(value));
            self.params.push(ParamEntry {
                key: key_input,
                value: value_input,
                enabled: *enabled,
            });
        }
        self.rebuild_param_subscriptions(window, cx);

        // 恢复 Headers
        for (key, value, enabled) in &state.headers {
            let key_input = cx.new(|cx| InputState::new(window, cx).default_value(key));
            let value_input = cx.new(|cx| InputState::new(window, cx).default_value(value));
            self.headers.push(HeaderEntry {
                key: key_input,
                value: value_input,
                enabled: *enabled,
            });
        }
        self.rebuild_header_subscriptions(window, cx);

        // 恢复 Auth 状态
        self.auth_state = match state.auth_type_index {
            1 => AuthState::Bearer(crate::ui::BearerTokenAuthData {
                token: cx.new(|cx| InputState::new(window, cx).default_value(&state.bearer_token)),
            }),
            2 => AuthState::Basic(crate::ui::BasicAuthData {
                username: cx
                    .new(|cx| InputState::new(window, cx).default_value(&state.basic_username)),
                password: cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(&state.basic_password)
                        .masked(true)
                }),
            }),
            3 => AuthState::ApiKey(crate::ui::ApiKeyAuthData {
                key: cx.new(|cx| InputState::new(window, cx).default_value(&state.api_key_name)),
                value: cx.new(|cx| InputState::new(window, cx).default_value(&state.api_key_value)),
                location: cx.new(|cx| {
                    let items = ApiKeyLocation::all();
                    SelectState::new(
                        items,
                        Some(IndexPath::new(state.api_key_location.to_index())),
                        window,
                        cx,
                    )
                }),
                location_value: state.api_key_location,
            }),
            _ => AuthState::NoAuth,
        };
        self.auth_type_select.update(cx, |s, cx| {
            s.set_selected_index(Some(IndexPath::new(state.auth_type_index)), window, cx);
        });

        // 恢复 Form Data
        self.body_state.form_data.clear();
        for e in &state.form_data {
            let key_input = cx.new(|cx| InputState::new(window, cx).default_value(&e.key));
            let type_select = FormDataParamType::create_select(window, cx);
            let value = if e.is_file {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(&e.file_path));
                FormDataValue::File(input, e.file_path.clone())
            } else if e.param_type == FormDataParamType::Boolean {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(&e.value));
                FormDataValue::Text(input)
            } else {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(&e.value));
                FormDataValue::Text(input)
            };
            self.body_state.form_data.push(FormDataEntry {
                key: key_input,
                value,
                enabled: e.enabled,
                param_type: e.param_type,
                type_select,
            });
        }

        // 恢复 URL-encoded Data
        self.body_state.urlencoded_data.clear();
        for e in &state.urlencoded_data {
            let key_input = cx.new(|cx| InputState::new(window, cx).default_value(&e.key));
            let type_select = FormDataParamType::create_select(window, cx);
            let value = if e.is_file {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(&e.file_path));
                FormDataValue::File(input, e.file_path.clone())
            } else {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(&e.value));
                FormDataValue::Text(input)
            };
            self.body_state.urlencoded_data.push(FormDataEntry {
                key: key_input,
                value,
                enabled: e.enabled,
                param_type: e.param_type,
                type_select,
            });
        }
        self.rebuild_form_data_type_subscriptions(window, cx);

        // 恢复脚本
        self.script_state.pre_request_script.update(cx, |s, cx| {
            s.set_value(&state.pre_request_script, window, cx);
        });
        self.script_state.test_script.update(cx, |s, cx| {
            s.set_value(&state.test_script, window, cx);
        });

        // 恢复设置
        self.settings = state.settings.clone();
        self.settings_inputs.timeout_input.update(cx, |s, cx| {
            s.set_value(&state.timeout_secs, window, cx);
        });
        self.settings_inputs.retry_input.update(cx, |s, cx| {
            s.set_value(&state.retry_count, window, cx);
        });

        // 恢复构建器标签
        self.builder_tab = state.builder_tab;

        // 恢复响应体展示
        if let Some(ref resp) = self.response {
            let resp_body = resp.body.clone();
            let resp_headers = resp.headers.clone();
            self.response_input.update(cx, |s, cx| {
                s.set_value(resp_body.as_ref(), window, cx);
            });
            self.response_xml_input.update(cx, |s, cx| {
                s.set_value(resp_body.as_ref(), window, cx);
            });
            self.response_text_input.update(cx, |s, cx| {
                s.set_value(resp_body.as_ref(), window, cx);
            });
            self.response_html_input.update(cx, |s, cx| {
                s.set_value(resp_body.as_ref(), window, cx);
            });
            self.rebuild_response_header_inputs(&resp_headers, window, cx);
        } else {
            self.response_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.response_xml_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.response_text_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.response_html_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.response_header_inputs.clear();
        }

        // 刷新美化编辑器
        self.update_pretty_editor(window, cx);

        // 恢复响应 Raw 格式选择器
        self.response_raw_format_select.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(state.response_raw_format.to_index())),
                window,
                cx,
            );
        });

        // 同步 last_synced_url
        self.last_synced_url = self.url_input.read(cx).value().to_string();

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
            self.body_state.raw_format = RawFormat::detect(content_type.as_deref(), &body_owned);
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
                    let value_entity =
                        cx.new(|cx| InputState::new(window, cx).default_value(&value));
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
    pub(crate) fn activate_environment(&mut self, env_id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        // Single lock acquisition to avoid re-entrant deadlock
        {
            let app = self.app_state.lock().unwrap();
            if let Err(e) = app.db.set_active_environment(env_id) {
                log::error!("设置活跃环境失败: {}", e);
                return;
            }
            if let Ok(Some(env)) = app.db.get_active_environment() {
                if let Err(e) = app.env_manager.load_from_json(&env.variables) {
                    log::warn!("加载环境变量失败: {}", e);
                }
            }
            // env_manager 为 Arc 共享，更新即时对 HttpClient 可见
            self.environments = Arc::new(app.db.get_environments().unwrap_or_default());
            self.active_environment_name = self
                .environments
                .iter()
                .find(|e| e.is_active)
                .map(|e| e.name.clone());
        }
        cx.notify();
    }

    /// 从数据库刷新环境列表
    fn load_environments(&mut self) {
        self.environments = Arc::new(self
            .app_state
            .lock()
            .unwrap()
            .db
            .get_environments()
            .unwrap_or_default());
        self.active_environment_name = self
            .environments
            .iter()
            .find(|e| e.is_active)
            .map(|e| e.name.clone());
    }

    /// 打开创建/编辑环境对话框
    pub(crate) fn open_environment_dialog(
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
        self.env_dialog_state
            .lock()
            .unwrap()
            .load_from(env_data.as_ref(), window, cx);
        cx.notify();
    }

    /// 打开全局变量编辑对话框
    pub(crate) fn open_global_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let globals = self.app_state.lock().unwrap().env_manager.get_all_globals();
        self.env_dialog_state
            .lock()
            .unwrap()
            .open_global(&globals, window, cx);
        cx.notify();
    }

    /// 获取当前活跃环境ID
    pub(crate) fn active_env_id(&self) -> Option<String> {
        self.environments
            .iter()
            .find(|e| e.is_active)
            .map(|e| e.id.clone())
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

    /// 关闭最上层的弹窗/浮层；返回是否真的关掉了东西（供 Esc 使用）
    ///
    /// 顺序即层级（从高到低）：右键菜单 > 代码生成 > 移动 > 文件夹 > 环境
    /// > 保存请求 > 快捷键浮层 > 设置浮层。一次只关一层，符合"Esc 逐层退出"的直觉。
    fn close_topmost_overlay(&mut self, cx: &mut Context<Self>) -> bool {
        fn open<T>(state: &Arc<Mutex<T>>, f: impl Fn(&T) -> bool) -> bool {
            state.lock().map(|s| f(&s)).unwrap_or(false)
        }

        if self.context_menu_target.is_some() {
            self.context_menu_target = None;
            self.context_menu_pos = None;
        } else if open(&self.code_gen_dialog_state, |s| s.open) {
            if let Ok(mut s) = self.code_gen_dialog_state.lock() {
                s.open = false;
            }
        } else if open(&self.move_dialog_state, |s| s.visible) {
            if let Ok(mut s) = self.move_dialog_state.lock() {
                s.visible = false;
            }
        } else if open(&self.folder_dialog_state, |s| s.visible) {
            if let Ok(mut s) = self.folder_dialog_state.lock() {
                s.visible = false;
            }
        } else if open(&self.env_dialog_state, |s| s.visible) {
            if let Ok(mut s) = self.env_dialog_state.lock() {
                s.visible = false;
            }
        } else if open(&self.save_request_dialog, |s| s.visible) {
            if let Ok(mut s) = self.save_request_dialog.lock() {
                s.visible = false;
                s.pending_request = None;
            }
        } else if self.show_shortcuts_popup {
            self.show_shortcuts_popup = false;
        } else if self.show_settings_popover {
            self.show_settings_popover = false;
        } else {
            return false;
        }
        cx.notify();
        true
    }

    /// 切换主题 — 更新 AppState、gpui_component 主题并持久化
    fn switch_theme(&mut self, theme: &str, cx: &mut Context<Self>) {
        self.app_state.lock().unwrap().set_theme(theme);
        self.cached_theme = Theme::from_str(theme);

        // 底色浅的主题要切到组件库的 Light 模式，否则组件对比度不对
        let mode = if Theme::is_light(theme) {
            gpui_component::theme::ThemeMode::Light
        } else {
            gpui_component::theme::ThemeMode::Dark
        };
        gpui_component::theme::Theme::change(mode, None, cx);

        cx.notify();
    }

    /// 保存当前请求到收藏
    pub fn save_current_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.url_input.read(cx).value().to_string();
        let method = self
            .method_select
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "GET".to_string());
        if url.trim().is_empty() {
            return;
        }
        // 从URL提取默认title
        let default_name = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(&url)
            .trim_end_matches('/');
        let short_name = if default_name.len() > 50 {
            format!("{}…", &default_name[..50])
        } else {
            default_name.to_string()
        };
        let headers_text = self
            .headers
            .iter()
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
            headers: if headers_text.is_empty() {
                None
            } else {
                Some(headers_text)
            },
            body: if body_text.is_empty() {
                None
            } else {
                Some(body_text)
            },
            description: None,
            folder_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.save_request_dialog
            .lock()
            .unwrap()
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
        self.collection_items = Arc::new(crate::ui::sidebar::build_collection_tree(
            &self.folders,
            &self.saved_requests,
            &self.expanded_folders,
        ));
        log::debug!(
            "集合树已重建: {} 个项目 ({} 个文件夹, {} 个请求)",
            self.collection_items.len(),
            self.folders.len(),
            self.saved_requests.len()
        );
    }

    /// 重新加载集合数据（从数据库）
    fn reload_collections(&mut self) {
        if let Ok(app) = self.app_state.lock() {
            self.saved_requests = Arc::new(app.db.get_saved_requests().unwrap_or_default());
            self.folders = Arc::new(app.db.get_folders().unwrap_or_default());
        }
        self.rebuild_collections();
    }

    /// 打开文件夹创建对话框
    pub fn open_folder_dialog(
        &mut self,
        parent_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("打开文件夹对话框, parent_id: {:?}", parent_id);
        self.folder_dialog_state
            .lock()
            .unwrap()
            .open_for_create(parent_id, window, cx);
        cx.notify();
    }

    /// 打开文件夹重命名对话框
    pub fn open_folder_edit_dialog(
        &mut self,
        folder_id: String,
        current_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_id = self
            .folders
            .iter()
            .find(|f| f.id == folder_id)
            .and_then(|f| f.parent_id.clone());
        self.folder_dialog_state.lock().unwrap().open_for_edit(
            folder_id,
            current_name,
            parent_id,
            window,
            cx,
        );
        cx.notify();
    }

    /// 重命名收藏请求
    pub fn open_request_rename_dialog(
        &mut self,
        request_id: &str,
        current_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.folder_dialog_state
            .lock()
            .unwrap()
            .open_for_request_rename(request_id.to_string(), current_name.to_string(), window, cx);
        cx.notify();
    }

    /// 删除文件夹
    pub fn delete_folder_from_sidebar(
        &mut self,
        folder_id: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(app) = self.app_state.lock() {
            if let Err(e) = app.db.delete_folder_cascade(folder_id) {
                log::error!("删除文件夹失败: id={}, err={}", folder_id, e);
            }
        }
        self.reload_collections();
        cx.notify();
    }

    /// 删除收藏请求
    pub fn delete_saved_request_from_sidebar(
        &mut self,
        request_id: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(app) = self.app_state.lock() {
            let _ = app.db.delete_saved_request(request_id);
        }
        self.reload_collections();
        cx.notify();
    }

    /// 打开移动对话框
    pub fn open_move_dialog(
        &mut self,
        item_id: &str,
        is_folder: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_dialog_state
            .lock()
            .unwrap()
            .open(item_id.to_string(), is_folder);
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
            app.db
                .get_saved_request_by_id(request_id)
                .ok()
                .flatten()
                .map(|req| (req.id, req.method, req.url, req.name, req.headers, req.body))
        });
        if let Some((id, m, u, n, h, b)) = found {
            self.load_saved_request(&id, &m, &u, &n, h.as_deref(), b.as_deref(), window, cx);
            return;
        }
        // fallback: use provided values
        self.load_saved_request(request_id, method, url, name, None, None, window, cx);
    }

    /// 检查 GitHub 上有没有新版本。
    ///
    /// 网络请求丢给 tokio 跑，结果通过 oneshot 回到 gpui 前台再刷界面 ——
    /// 保证检查期间界面不卡（gpui 前台执行器里不能阻塞）。
    pub(crate) fn check_for_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use crate::app::updater::{self, UpdateStatus};

        if self.update_status.is_checking() {
            return;
        }
        self.update_status = UpdateStatus::Checking;
        self.update_checked_at = Some(std::time::Instant::now());
        cx.notify();

        // 检查更新也该走应用里配的代理，否则挂了代理的用户永远查不到
        let proxy = {
            let app = self.app_state.lock().unwrap();
            if app.config.proxy.enabled && !app.config.proxy.url.trim().is_empty() {
                Some(app.config.proxy.url.clone())
            } else {
                None
            }
        };
        let rt = self.app_state.lock().unwrap().rt_handle.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        rt.spawn(async move {
            let _ = tx.send(updater::check(proxy).await);
        });

        cx.spawn_in(window, async move |this: WeakEntity<MainView>, cx| {
            let status = match rx.await {
                Ok(status) => status,
                Err(_) => UpdateStatus::Failed("检查任务被中断".to_string()),
            };
            let _ = this.update_in(cx, |this, _window, cx| {
                this.update_status = status;
                cx.notify();
            });
        })
        .detach();
    }

    /// 检查所有对话框刷新标志
    fn check_dialog_refresh_flags(&mut self) {
        // 环境变量对话框
        if self.env_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 环境变量对话框标记");
            self.load_environments();
            self.saved_requests = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_saved_requests()
                .unwrap_or_default());
            self.folders = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_folders()
                .unwrap_or_default());
            self.env_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 文件夹对话框
        if self.folder_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 文件夹对话框标记");
            self.folders = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_folders()
                .unwrap_or_default());
            self.saved_requests = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_saved_requests()
                .unwrap_or_default());
            self.folder_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 移动对话框
        if self.move_dialog_state.lock().unwrap().needs_refresh {
            log::debug!("刷新: 移动对话框标记");
            self.folders = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_folders()
                .unwrap_or_default());
            self.saved_requests = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_saved_requests()
                .unwrap_or_default());
            self.move_dialog_state.lock().unwrap().needs_refresh = false;
            self.needs_collections_refresh = true;
        }
        // 拖拽放置
        if self
            .needs_drop_refresh
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            log::debug!("刷新: 拖拽放置标记");
            self.folders = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_folders()
                .unwrap_or_default());
            self.saved_requests = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_saved_requests()
                .unwrap_or_default());
            self.needs_collections_refresh = true;
        }
        // 保存到收藏夹对话框
        if self.save_request_dialog.lock().unwrap().needs_refresh {
            log::debug!("刷新: 保存到收藏夹对话框标记");
            self.saved_requests = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_saved_requests()
                .unwrap_or_default());
            self.folders = Arc::new(self
                .app_state
                .lock()
                .unwrap()
                .db
                .get_folders()
                .unwrap_or_default());
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

/// 设置面板里的「关于」区：本地版本 + 从 GitHub Releases 检查更新
///
/// 状态来自 MainView.update_status，由 check_for_updates 异步写入。
fn update_section(this: &MainView, cx: &mut Context<MainView>, theme: &Theme) -> gpui::Div {
    use crate::app::updater::{UpdateStatus, CURRENT_VERSION};

    let t_current = this.t("update.current");
    let t_check = this.t("update.check");
    let t_recheck = this.t("update.recheck");
    let t_up_to_date = this.t("update.up_to_date");
    let t_available = this.t("update.available");
    let t_download = this.t("update.download");
    let t_release_page = this.t("update.release_page");
    let t_failed = this.t("update.failed");
    let t_hint = this.t("update.hint");
    let t_published = this.t("update.published");
    let checking = this.update_status.is_checking();
    // 按钮文案：检查中显示"正在检查"，查过一次显示"重新检查"，没查过显示"检查更新"
    let has_checked = !matches!(this.update_status, UpdateStatus::Idle);
    let check_label = if checking {
        this.t("update.checking")
    } else if has_checked {
        t_recheck.clone()
    } else {
        t_check.clone()
    };

    // 状态行：图标 + 文案（颜色区分成功/失败/有新版本）
    let (icon, icon_color, text, text_color) = match &this.update_status {
        UpdateStatus::Idle => (
            IconName::Info,
            theme.muted_foreground,
            t_hint.clone(),
            theme.muted_foreground,
        ),
        UpdateStatus::Checking => (
            IconName::LoaderCircle,
            theme.accent,
            this.t("update.checking"),
            theme.muted_foreground,
        ),
        UpdateStatus::UpToDate { latest } => (
            IconName::Check,
            theme.success,
            SharedString::from(format!("{t_up_to_date}（v{latest}）")),
            theme.muted_foreground,
        ),
        UpdateStatus::Available(info) => (
            IconName::TriangleAlert,
            theme.accent,
            SharedString::from(format!("{t_available} v{}", info.version())),
            theme.foreground,
        ),
        UpdateStatus::Failed(err) => (
            IconName::Close,
            theme.error,
            SharedString::from(format!("{t_failed}：{err}")),
            theme.error,
        ),
    };

    // 有新版本时补一行：发布日期 + 本平台安装包大小
    let extra = match &this.update_status {
        UpdateStatus::Available(info) => {
            let mut parts: Vec<String> = Vec::new();
            if !info.published_date().is_empty() {
                parts.push(format!("{} {}", t_published, info.published_date()));
            }
            if let Some(asset) = info.asset_for_platform() {
                parts.push(format!("{} · {}", asset.name, asset.size_text()));
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("　"))
            }
        }
        _ => None,
    };

    // 下载按钮：优先本平台安装包，没有再退化成打开发布页
    let download_url = match &this.update_status {
        UpdateStatus::Available(info) => Some(
            info.asset_for_platform()
                .map(|a| a.browser_download_url.clone())
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| info.html_url.clone()),
        ),
        _ => None,
    };
    let release_url = match &this.update_status {
        UpdateStatus::Available(info) if !info.html_url.is_empty() => Some(info.html_url.clone()),
        _ => None,
    };

    div()
        .flex()
        .flex_col()
        .gap(px(GAP_S))
        // 当前版本
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.muted_foreground)
                        .child(t_current),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight(600.0))
                        .text_color(theme.foreground)
                        .child(format!("v{CURRENT_VERSION}")),
                ),
        )
        // 状态
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(GAP_XS + 2.0))
                .child(Icon::new(icon).xsmall().text_color(icon_color))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_size(px(11.0))
                        .text_color(text_color)
                        .child(text),
                ),
        )
        .when_some(extra, |d, extra| {
            d.child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.muted_foreground)
                    .child(extra),
            )
        })
        // 操作按钮
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(GAP_S))
                .flex_wrap()
                .child(
                    ghost_button(
                        "update-check-btn",
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(GAP_XS))
                            .child(Icon::new(IconName::Replace).xsmall())
                            .child(check_label.clone()),
                        theme,
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                            this.check_for_updates(window, cx);
                        }),
                    ),
                )
                .when_some(release_url, |d, url| {
                    d.child(
                        ghost_button(
                            "update-release-btn",
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(GAP_XS))
                                .child(Icon::new(IconName::ExternalLink).xsmall())
                                .child(t_release_page.clone()),
                            theme,
                        )
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            cx.open_url(&url);
                        }),
                    )
                })
                .when_some(download_url, |d, url| {
                    d.child(
                        primary_button_sm(
                            "update-download-btn",
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(2.0))
                                .child(Icon::new(IconName::ArrowDown).xsmall())
                                .child(t_download.clone()),
                            theme,
                        )
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            cx.open_url(&url);
                        }),
                    )
                }),
        )
}

/// 设置浮层面板 — 在侧边栏 logo 下方展开
fn settings_popover(
    this: &mut MainView,
    cx: &mut Context<MainView>,
    theme: &Theme,
    max_height: f32,
) -> gpui::Div {
    let current_lang = this
        .app_state
        .lock()
        .unwrap()
        .config
        .general
        .language
        .clone();
    let current_theme = this.app_state.lock().unwrap().config.general.theme.clone();
    let auto_save = this.app_state.lock().unwrap().config.general.auto_save;
    let proxy_enabled = this.app_state.lock().unwrap().config.proxy.enabled;
    let t_lang_title = this.t("language.title");
    let t_lang_zh = this.t("language.zh");
    let t_lang_en = this.t("language.en");
    let t_theme_title = this.t("theme.title");
    let t_settings_general = this.t("settings.general");
    let t_settings_auto_save = this.t("settings.auto_save");
    let t_settings_proxy = this.t("settings.proxy");
    let t_settings_proxy_enable = this.t("settings.proxy_enable");
    let t_settings_proxy_url = this.t("settings.proxy_url");
    let t_shortcuts = this.t("settings.shortcuts");
    let title = this.t("settings.title");
    // 主题名（12 个）按 4 列 × 3 行排版，宽高一致
    let theme_items: Vec<(SharedString, &'static str, String)> = vec![
        (this.t("theme.dark"), "theme-dark", "dark".to_string()),
        (this.t("theme.light"), "theme-light", "light".to_string()),
        (this.t("theme.sepia"), "theme-sepia", "sepia".to_string()),
        (this.t("theme.ocean"), "theme-ocean", "ocean".to_string()),
        (this.t("theme.sunset"), "theme-sunset", "sunset".to_string()),
        (this.t("theme.forest"), "theme-forest", "forest".to_string()),
        (this.t("theme.monokai"), "theme-monokai", "monokai".to_string()),
        (this.t("theme.nord"), "theme-nord", "nord".to_string()),
        (this.t("theme.dracula"), "theme-dracula", "dracula".to_string()),
        (this.t("theme.tokyonight"), "theme-tokyonight", "tokyonight".to_string()),
        (this.t("theme.gruvbox"), "theme-gruvbox", "gruvbox".to_string()),
        (this.t("theme.latte"), "theme-latte", "latte".to_string()),
    ];
    let shortcut_rows: Vec<(&'static str, SharedString)> = vec![
        ("Ctrl+Enter / Ctrl+S", this.t("settings.shortcuts.send")),
        ("Ctrl+N", this.t("settings.shortcuts.new_tab")),
        ("Ctrl+W", this.t("settings.shortcuts.close_tab")),
        ("Ctrl+H", this.t("settings.shortcuts.history")),
        ("Ctrl+E", this.t("settings.shortcuts.env")),
        ("Ctrl+T", this.t("settings.shortcuts.theme")),
        ("Ctrl+L", this.t("settings.shortcuts.lang")),
    ];

    // ---- 各区块，统一 push 成 AnyElement 再交给 flex 容器 ----
    let mut sections: Vec<AnyElement> = Vec::new();

    // 语言
    sections.push(section_title(t_lang_title, theme).into_any_element());
    // 注意：两个选项按钮的闭包类型不同，必须用两次 .child()，放进同一个数组会因类型不同编译失败
    sections.push(
        div()
            .flex()
            .flex_row()
            .gap(px(GAP_S))
            .child(setting_option_btn(
                &t_lang_zh,
                "lang-zh",
                current_lang == "zh-CN",
                theme,
                cx,
                |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                    this.app_state.lock().unwrap().switch_language("zh-CN");
                    this.refresh_translations();
                    cx.notify();
                },
            ))
            .child(setting_option_btn(
                &t_lang_en,
                "lang-en",
                current_lang == "en-US",
                theme,
                cx,
                |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                    this.app_state.lock().unwrap().switch_language("en-US");
                    this.refresh_translations();
                    cx.notify();
                },
            ))
            .into_any_element(),
    );

    // 主题
    sections.push(section_divider(theme).into_any_element());
    sections.push(section_title(t_theme_title, theme).into_any_element());
    for (row_index, chunk) in theme_items.chunks(4).enumerate() {
        // 两行主题按钮之间留出小间距（容器 gap 在滚动包装下不生效，这里显式给）
        let mut row = div().flex().flex_row().gap(px(GAP_S));
        if row_index > 0 {
            row = row.mt(px(GAP_XS + 2.0));
        }
        for (label, id, value) in chunk {
            let value = value.clone();
            let active = current_theme == value;
            row = row.child(setting_option_btn(
                label,
                id,
                active,
                theme,
                cx,
                move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                    this.switch_theme(&value, cx);
                },
            ));
        }
        sections.push(row.into_any_element());
    }

    // 常规
    sections.push(section_divider(theme).into_any_element());
    sections.push(section_title(t_settings_general, theme).into_any_element());
    sections.push(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(30.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.foreground)
                    .child(t_settings_auto_save.clone()),
            )
            .child(
                crate::ui::components::toggle_switch("auto-save", auto_save, theme).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(
                        |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                            let new_val = !this.app_state.lock().unwrap().config.general.auto_save;
                            {
                                let mut state = this.app_state.lock().unwrap();
                                Arc::make_mut(&mut state.config).general.auto_save = new_val;
                                let _ = state.config.save();
                            }
                            this.update_auto_save_task(window, cx);
                            cx.notify();
                        },
                    ),
                ),
            )
            .into_any_element(),
    );

    // 代理
    sections.push(section_divider(theme).into_any_element());
    sections.push(section_title(t_settings_proxy, theme).into_any_element());
    sections.push(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(30.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.foreground)
                    .child(t_settings_proxy_enable.clone()),
            )
            .child(
                crate::ui::components::toggle_switch("proxy-enabled", proxy_enabled, theme)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(
                            |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                let new_val = !this.app_state.lock().unwrap().config.proxy.enabled;
                                {
                                    let mut state = this.app_state.lock().unwrap();
                                    Arc::make_mut(&mut state.config).proxy.enabled = new_val;
                                    let _ = state.config.save();
                                }
                                let url = this.proxy_url_input.read(cx).value().to_string();
                                this.app_state.lock().unwrap().update_proxy(new_val, &url);
                                cx.notify();
                            },
                        ),
                    ),
            )
            .into_any_element(),
    );
    sections.push(
        div()
            .mt(px(GAP_S))
            .flex()
            .flex_col()
            .gap(px(GAP_XS + 2.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(GAP_XS + 2.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.muted_foreground)
                            .child(t_settings_proxy_url.clone()),
                    )
                    .child(
                        div()
                            .id("proxy-tips-icon")
                            .w(px(14.0))
                            .h(px(14.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(if this.proxy_tips_hovered {
                                theme.accent
                            } else {
                                theme.muted_foreground
                            })
                            .text_size(px(10.0))
                            .text_color(if this.proxy_tips_hovered {
                                theme.accent
                            } else {
                                theme.muted_foreground
                            })
                            .cursor_default()
                            .on_mouse_move(cx.listener(
                                |this, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    cx.stop_propagation();
                                    this.proxy_tips_hovered = true;
                                    this.proxy_tips_x = Some(e.position.x.into());
                                    this.proxy_tips_y = Some(e.position.y.into());
                                    cx.notify();
                                },
                            ))
                            .child("?"),
                    ),
            )
            .child(
                Input::new(&this.proxy_url_input)
                    .h(px(30.0))
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.input_background)
                    .text_color(theme.foreground),
            )
            .into_any_element(),
    );

    // 快捷键（默认折叠）
    sections.push(section_divider(theme).into_any_element());
    let mut shortcuts = div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(GAP_XS + 2.0))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                        this.show_shortcuts_popup = !this.show_shortcuts_popup;
                        cx.notify();
                    }),
                )
                .child(
                    Icon::new(if this.show_shortcuts_popup {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .xsmall()
                    .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight(600.0))
                        .text_color(theme.muted_foreground)
                        .child(t_shortcuts),
                ),
        );
    if this.show_shortcuts_popup {
        let mut rows = div().mt(px(GAP_S)).flex().flex_col().gap(px(GAP_XS));
        for (key, label) in shortcut_rows {
            rows = rows.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(GAP_S))
                    .child(
                        div()
                            .min_w(px(112.0))
                            .px(px(GAP_XS + 2.0))
                            .py(px(2.0))
                            .rounded_sm()
                            .bg(theme.input_background)
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(10.0))
                            .text_color(theme.muted_foreground)
                            .child(key),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.muted_foreground)
                            .child(label),
                    ),
            );
        }
        shortcuts = shortcuts.child(rows);
    }
    sections.push(shortcuts.into_any_element());

    // 关于 / 更新检查
    sections.push(section_divider(theme).into_any_element());
    sections.push(section_title(this.t("settings.about"), theme).into_any_element());
    sections.push(update_section(this, cx, theme).into_any_element());

    div()
        .w(px(300.0))
        .max_h(px(max_height))
        .flex_col()
        // 裁剪：滚动区里的段落比可视区高，若无裁剪会画到面板之外，
        // 那些段落本身没有底色 → 看起来就是「弹窗里能看到下层内容」
        .overflow_hidden()
        .child(
            // 标题栏固定，内容区可滚动：窗口再矮也不会被截断
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .h(px(38.0))
                .px(px(GAP_M))
                .flex_shrink_0()
                .border_b(px(1.0))
                .border_color(theme.border)
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight(600.0))
                        .text_color(theme.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .id("settings-close")
                        .w(px(24.0))
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .cursor_pointer()
                        .text_color(theme.muted_foreground)
                        .hover(|s| s.bg(theme.muted_background).text_color(theme.foreground))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                this.show_settings_popover = false;
                                cx.notify();
                            }),
                        )
                        .child(Icon::new(IconName::Close).xsmall()),
                ),
        )
        .child(
            div()
                .id("settings-body")
                .flex_col()
                // 必须给滚动区**显式高度**：max_h 只约束外框，容器自身高度仍等于
                // 内容高度，gpui 按容器高度算滚动范围 → 范围恒为 0（探针实测
                // 容器 511px == 内容 511px，就是"没法滚动"）
                // 38=标题栏，2=上下边框；不给足会被边框挤出去一点点
                .h(px((max_height - 40.0).max(120.0)))
                // 滚动区自带不透明底色：即使某处漏裁剪，也不会透出下层
                .bg(theme.background)
                // 抵消 flex 的自动最小高度，否则内容会把容器撑开、滚动范围归零
                .min_h(px(0.0))
                // 必须用 gpui 原生滚动：gpui_component 的 overflow_y_scrollbar
                // 会给内容注入 size_auto + flex_1，滚动范围会被算成 0（就是"没法滚动"）
                .overflow_y_scroll()
                .child(
                    div()
                        .flex_none()
                        .w_full()
                        .flex_col()
                        .children(sections),
                ),
        )
}

fn setting_option_btn(
    label: &str,
    id: &'static str,
    active: bool,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_toggle: impl Fn(&mut MainView, &MouseDownEvent, &mut Window, &mut Context<MainView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .text_xs()
        .cursor_pointer()
        .min_w(px(60.0))
        .px_2p5()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(if active { theme.accent } else { theme.border })
        .font_weight(if active { FontWeight(600.0) } else { FontWeight(400.0) })
        .bg(if active {
            theme.accent
        } else {
            theme.muted_background
        })
        .text_color(if active {
            theme.accent_foreground
        } else {
            theme.muted_foreground
        })
        .hover(|s| if active { s } else { s.bg(theme.muted_background).border_color(theme.muted_foreground) })
        .on_mouse_down(MouseButton::Left, cx.listener(on_toggle))
        .child(label.to_string())
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_width = if self.sidebar_collapsed {
            px(48.0)
        } else {
            px(280.0)
        };
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
        let theme = self.cached_theme.clone();
        // 侧边栏内容区高度：窗口高度减掉顶部/底部 chrome。
        // 侧边栏整条高度链是 auto，滚动区必须显式定高，否则滚动范围会算成 0
        let sidebar_content_h = (window.bounds().size.height.as_f32() - 121.0).max(160.0);
        // 请求标签条：标签定宽，标签过多时整体横向滚动，并据此决定是否显示左右箭头
        let tabs_content_w = (REQUEST_TAB_WIDTH + REQUEST_TAB_GAP)
            * self.request_tabs.len() as f32
            - REQUEST_TAB_GAP;
        let tabs_max = (tabs_content_w - self.tabs_viewport).max(0.0);
        let tabs_offset = f32::from(self.tabs_scroll.offset().x);
        let tabs_overflow = tabs_max > 0.5;
        // 配置开关（代理 / 自动保存）：渲染期间只读，锁一次取出来
        let (proxy_on, auto_save_on) = {
            let state = self.app_state.lock().unwrap();
            (state.config.proxy.enabled, state.config.general.auto_save)
        };

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .relative()
            .track_focus(&self.root_focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                // Esc 逐层关闭弹窗/浮层（一次一层，没有可关的就不拦截事件）
                if event.keystroke.key == "escape"
                    && !event.keystroke.modifiers.control
                    && this.close_topmost_overlay(cx)
                {
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.modifiers.control {
                    match event.keystroke.key.as_str() {
                        "enter" | "s" => {
                            cx.stop_propagation();
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
                            cx.stop_propagation();
                            this.add_tab(window, cx);
                        }
                        "w" => {
                            cx.stop_propagation();
                            if this.request_tabs.len() > 1 {
                                let tab_idx = this.active_tab;
                                this.close_tab(tab_idx, window, cx);
                            }
                        }
                        "h" => {
                            cx.stop_propagation();
                            this.set_sidebar_tab(SidebarTab::History, cx);
                        }
                        "e" => {
                            cx.stop_propagation();
                            this.set_sidebar_tab(SidebarTab::Environments, cx);
                        }
                        "t" => {
                            cx.stop_propagation();
                            const THEMES: &[&str] = Theme::NAMES;
                            let current = this.app_state.lock().unwrap().config.general.theme.clone();
                            let idx = THEMES.iter().position(|t| *t == current).unwrap_or(0);
                            let next = THEMES[(idx + 1) % THEMES.len()];
                            this.switch_theme(next, cx);
                        }
                        "l" => {
                            cx.stop_propagation();
                            let current = this.app_state.lock().unwrap().config.general.language.clone();
                            let next = if current == "zh-CN" { "en-US" } else { "zh-CN" };
                            this.app_state.lock().unwrap().switch_language(next);
                            this.refresh_translations();
                            cx.notify();
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
                            // 侧边栏宽度必须固定：否则主工作区（请求标签栏等）的 min-content
                            // 过大时会把侧边栏压缩，导致收藏夹/历史/环境变量切换时宽度不一致
                            .flex_shrink_0()
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
                                    .when(self.sidebar_collapsed, |s| s.justify_center())
                                    .when(!self.sidebar_collapsed, |s| s.justify_between().px_3())
                                    .border_b(px(1.0))
                                    .border_color(theme.muted_background)
                                    .children([
                                        if !self.sidebar_collapsed {
                                            div().text_color(rgb(0xf97316)).font_semibold().child("ApiPost")
                                        } else {
                                            div()
                                        },
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
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>| {
                                                this.show_settings_popover = !this.show_settings_popover;
                                                // 打开设置面板时顺手查一次更新，10 分钟内不重复打 GitHub
                                                if this.show_settings_popover {
                                                    let stale = this
                                                        .update_checked_at
                                                        .map(|at| at.elapsed().as_secs() >= 600)
                                                        .unwrap_or(true);
                                                    if stale {
                                                        this.check_for_updates(window, cx);
                                                    }
                                                }
                                                cx.notify();
                                            }))
                                            .child(
                                                Icon::new(IconName::Settings).small().text_color(theme.muted_foreground)
                                            ),
                                    ]),
                                // 标签页按钮
                                div()
                                    .flex()
                                    // 设置浮层盖住的就是这块区域：浮层挂在侧边栏子树里，
                                    // 标签行排在它之后绘制，会压在浮层上面（表现为"弹窗里还看得见
                                    // 收藏夹/历史/环境变量图标"）。让它在浮层打开时不可见即可 ——
                                    // 区域本就被浮层完全遮住，视觉上无差别，且不依赖绘制顺序。
                                    .opacity(if self.show_settings_popover { 0.0 } else { 1.0 })
                                    .when(self.sidebar_collapsed, |s| s.flex_col().flex_1())
                                    .when(!self.sidebar_collapsed, |s| s.flex_row().h(px(40.0)))
                                    .children([
                                        div()
                                            .id("sidebar-collections-tab") // 收藏夹
                                            .w(px(48.0))
                                            .when(self.sidebar_collapsed, |s| s.h(px(48.0)))
                                            .when(!self.sidebar_collapsed, |s| s.h(px(40.0)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Collections { theme.accent } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::Collections { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Collections, cx);
                                            }))
                                            .child(Icon::new(IconName::FolderClosed).small()),
                                        div()
                                            .id("sidebar-history") // 历史记录
                                            .w(px(48.0))
                                            .when(self.sidebar_collapsed, |s| s.h(px(48.0)))
                                            .when(!self.sidebar_collapsed, |s| s.h(px(40.0)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::History { theme.accent } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::History { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::History, cx);
                                            }))
                                            .child(Icon::new(IconName::GalleryVerticalEnd).small()),
                                        div()
                                            .id("sidebar-env-tab")
                                            .w(px(48.0))
                                            .when(self.sidebar_collapsed, |s| s.h(px(48.0)))
                                            .when(!self.sidebar_collapsed, |s| s.h(px(40.0)))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .bg(if sidebar_tab == SidebarTab::Environments { theme.accent } else { theme.input_background })
                                            .text_color(if sidebar_tab == SidebarTab::Environments { theme.accent_foreground } else { theme.muted_foreground })
                                            .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                this.set_sidebar_tab(SidebarTab::Environments, cx);
                                            }))
                                            .child(Icon::new(IconName::Globe).small()),
                                    ]),
                                // 侧边栏内容
                                if !self.sidebar_collapsed {
                                    // 注意：这里必须是 flex_col。
                                    // 用 flex()（row）时，子元素的 flex_1 作用于宽度，
                                    // 高度只剩「内容高度」，里面的 overflow_y_scroll 就没有
                                    // 可滚动边界了 —— 表现就是列表滚不动。
                                    div()
                                        .flex_col()
                                        .h(px(sidebar_content_h))
                                        // 必须解除 flex 子项的自动最小高度(=内容高度)，
                                        // 否则显式高度会被内容顶开，列表超出窗口被裁掉且滚不动
                                        .min_h(px(0.0))
                                        .flex_shrink_0()
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
                                                        .into_any_element()
                                                } else {
                                                    // 滚动容器：确定高度 + 原生滚动（gpui 自带滚动条）
                                                    div()
                                                        .id("history-scroll")
                                                        .size_full()
                                                        .flex_col()
                                                        .overflow_y_scroll()
                                                        .child(
                                                            // 内容：自然高度，绝不能被 flex_shrink 压缩
                                                            div()
                                                                .id("history-list")
                                                                .w_full()
                                                                .flex_none()
                                                                .flex_col()
                                                                .gap_1()
                                                                .p_2().pb(px(24.0))
                                                                .children(history.iter().enumerate().map(|(history_idx, entry)| {
let method_clr = method_color(&entry.method);
                                                            // 每行只做一次 Arc 引用计数，不再 clone 整个 entry 和响应体：
                                                            // 这些数据只在点击时才需要，没必要每帧复制
                                                            let history_row = self.history.clone();
                                                            let entry_response_time_ms = entry.response_time_ms;
                                                            let entry_response_size = entry.response_size.or_else(|| entry.response_body.as_ref().map(|b| b.len() as i64));
                                                            let display_method = entry.method.clone();
                                                            let display_url = entry.url.clone();
                                                            div()
                                                                .w_full()
                                                                // 关键：flex 子项默认 flex_shrink=1，会被压缩到刚好装下，
                                                                // 内容高度等于容器高度就永远滚不动
                                                                .flex_none()
                                                                .flex_col()
                                                                .gap_1()
                                                                .p_2()
                                                                .rounded_md()
                                                                .cursor_pointer()
                                                                .hover(|s| s.bg(theme.code_background)).bg(theme.muted_background)
                                                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                    // 从共享的 Arc 里按下标取，避免每帧复制大对象
                                                                    let Some(entry) = history_row.get(history_idx) else { return; };
                                                                    this.load_saved_request(
                                                                        &entry.id,
                                                                        &entry.method,
                                                                        &entry.url,
                                                                        "",
                                                                        entry.headers.as_deref(),
                                                                        entry.body.as_deref(),
                                                                        _window,
                                                                        cx,
                                                                    );

                                                                    if let Some(status) = entry.response_status {
                                                                        let resp_body = entry.response_body.clone().unwrap_or_default();
                                                                        let resp_headers: std::collections::HashMap<String, String> = entry.response_headers.as_ref().and_then(|h| serde_json::from_str(h).ok()).unwrap_or_default();
                                                                        let content_type = resp_headers.get("content-type").cloned();
                                                                        // 在 resp_headers 被移动前重建 header inputs
                                                                        this.rebuild_response_header_inputs(&resp_headers, _window, cx);
                                                                        this.response_raw_format = RawFormat::detect(content_type.as_deref(), &resp_body);
                                                                        this.response_input.update(cx, |state, cx| {
                                                                            state.set_value(&resp_body, _window, cx);
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
                                                                        // resp_body move 进 Arc，避免 clone
                                                                        let response = HttpResponse {
                                                                            status: status as u16,
                                                                            headers: resp_headers,
                                                                            body: Arc::from(resp_body),
                                                                            raw_body: None,
                                                                            time_ms: entry_response_time_ms.unwrap_or(0),
                                                                            size_bytes: entry_response_size.unwrap_or(0),
                                                                            cookies: Vec::new(),
                                                                        };
                                                                        this.response = Some(Arc::new(response));
                                                                        this.update_pretty_editor(_window, cx);
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
                                                                        this.update_pretty_editor(_window, cx);
                                                                    }
                                                                }))
                                                                .children([
                                                                    div().flex().items_center().gap_2().children([
                                                                        div().px_1().py_px().rounded_sm().bg(rgb(method_clr))
                                                                            .text_xs().text_color(theme.accent_foreground)
                                                                            .child(display_method),
                                                                        div().flex_1().min_w(px(0.0)).truncate().text_xs().text_color(theme.foreground)
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
                                                        )
                                                        .into_any_element()
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
                                                                    .child(format!("{} ({})", self.t("sidebar.collections"), collection_items.len())),
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
                                                            let drop_app = self.app_state.clone();
                                                            let drop_flag = self.needs_drop_refresh.clone();
                                                            let drop_eid = cx.entity_id();
                                                            div()
                                                                .id("sidebar-collections-scroll")
                                                                // 直接给显式高度：这条高度链上 flex_1/100% 都拿不到父容器的确定高度
                                                                .h(px(sidebar_content_h))
                                                                .min_h(px(0.0))
                                                                .flex_shrink_0()
                                                                .flex_col()
                                                                // 原生滚动（见历史记录处的说明：Scrollable 会把内容
                                                                // 压成容器高度，可滚动范围变成 0）
                                                                .overflow_y_scroll()
                                                                .overflow_x_hidden()
                                                                .drag_over::<DragItem>(|style, _data, _window, _cx| {
                                                                    style.bg(rgba(0x88888822))
                                                                })
                                                                .on_drop::<DragItem>(move |data: &DragItem, _window, cx| {
                                                                    if let Ok(app) = drop_app.lock() {
                                                                        if data.is_folder {
                                                                            let _ = app.db.move_folder(&data.id, None);
                                                                        } else {
                                                                            let _ = app.db.move_request(&data.id, None);
                                                                        }
                                                                    }
                                                                    drop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                                                                    cx.notify(drop_eid);
                                                                })
                                                                .child(
                                                                    // 内容层：自然高度
                                                                    div()
                                                                        .id("sidebar-collections-list")
                                                                        .w_full()
                                                                        .flex_none()
                                                                        .flex_col()
                                                                        .p_2().pb(px(40.0))
                                                                        .child(render_collection_panel(&collection_items, &self.context_menu_target, &self.hovered_item_name, cx, &theme, &self.app_state, self.needs_drop_refresh.clone())),
                                                                )
                                                                .into_any_element()
                                                        }
                                                    )
                                                                        .into_any_element()
                                            } else {
                                                // 环境列表已抽到 src/ui/sidebar/environment_panel.rs
                                                render_environment_panel(
                                                    &environments,
                                                    self.active_env_id(),
                                                    &self.app_state,
                                                    &theme,
                                                    sidebar_content_h,
                                                    cx,
                                                )
                                                                        .into_any_element()
                                            }
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
                            // 设置浮层面板（绝对定位，贴在侧边栏左上角）
                            if self.show_settings_popover {
                                settings_popover(self, cx, &theme, window.bounds().size.height.as_f32() - 64.0)
                                    .absolute()
                                    .top(px(48.0))
                                    .left(px(0.0))
                                    .w(px(290.0))
                                    .shadow_md()
                                    .occlude()
                            } else {
                                div()
                            },
                            ]),
                        // ==================== 主工作区 ====================
                        div()
                            .relative()
                            .flex_1()
                            // min_w(0)：主工作区的 min-content（请求标签栏等定宽内容）不应向上传递，
                            // 否则窗口变窄时会撑破整行、把侧边栏挤窄或裁掉右侧按钮
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                this.context_menu_target = None;
                                cx.notify();
                            }))
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                if this.splitter_dragging {
                                    let y: f32 = event.position.y.into();
                                    if this.update_splitter_drag(y) {
                                        cx.notify();
                                    }
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
                                    .h(px(36.0))
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .items_end()
                                    .bg(theme.muted_background)
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    // 标签条：每个标签定宽，标签过多时整条左右滚动（滚轮即可横向滚动）
                                    .child(
                                        div()
                                            .id("request-tabs-scroll")
                                            .flex_1()
                                            .min_w(px(0.0))
                                            // 用显式高度（而非 h_full）：滚动容器的 hitbox 必须覆盖指针位置，
                                            // 否则 gpui 不会把滚轮事件交给它
                                            .h(px(36.0))
                                            .flex()
                                            .flex_row()
                                            .items_end()
                                            .gap_px()
                                            .overflow_x_scroll()
                                            .track_scroll(&self.tabs_scroll)
                                            .children(request_tabs.iter().enumerate().map(|(i, tab)| {
                                                let is_active = i == active_tab;
                                                let method_clr = method_color(&tab.method);
                                                let tab_method = tab.method.clone();
                                                // 如果是默认标签名称，使用i18n
                                                let tab_display_name: SharedString = if tab.name == "新建请求" || tab.name == "New Request" {
                                                    self.t("sidebar.new_request")
                                                } else {
                                                    SharedString::from(tab.name.clone())
                                                };
                                                let show_close = show_close;

                                                div()
                                                    .h(px(36.0))
                                                    .w(px(REQUEST_TAB_WIDTH))
                                                    // 标签定宽且不收缩：标签过多时由标签条横向滚动，
                                                    // 而不是把每个标签压窄
                                                    .flex_shrink_0()
                                                    .pl_3()
                                                    .pr_1()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .bg(if is_active { theme.background } else { rgba(0x00000000) })
                                                    .rounded_t_md()
                                                    .border_b_2()
                                                    .border_b(if is_active { px(2.0) } else { px(0.0) })
                                                    .border_color(if is_active { theme.accent } else { rgba(0x00000000) })
                                                    .hover(|s| if is_active { s } else { s.bg(theme.code_background) })
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
                                                                    .text_color(if is_active { theme.foreground } else { theme.muted_foreground })
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
                                    )
                                    // 左右滚动按钮：只在标签溢出时出现，某方向没有更多标签时置灰不可点
                                    // （鼠标滚轮悬停在标签条上也可以直接横向滚动）
                                    .when(tabs_overflow, |el| {
                                        let can_left = tabs_offset > 0.5;
                                        let can_right = tabs_offset < tabs_max - 0.5;
                                        el.child(
                                            div()
                                                .id("tabs-scroll-left")
                                                .h(px(36.0))
                                                .w(px(TABS_SCROLL_BUTTON_WIDTH))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_color(if can_left { theme.foreground } else { theme.border })
                                                .when(can_left, |btn| {
                                                    btn.cursor_pointer()
                                                        .hover(|s| s.bg(theme.code_background))
                                                        .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                            this.scroll_tabs_by(-1);
                                                            cx.notify();
                                                        }))
                                                })
                                                .child(Icon::new(IconName::ChevronLeft).small()),
                                        )
                                        .child(
                                            div()
                                                .id("tabs-scroll-right")
                                                .h(px(36.0))
                                                .w(px(TABS_SCROLL_BUTTON_WIDTH))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_color(if can_right { theme.foreground } else { theme.border })
                                                .when(can_right, |btn| {
                                                    btn.cursor_pointer()
                                                        .hover(|s| s.bg(theme.code_background))
                                                        .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                            this.scroll_tabs_by(1);
                                                            cx.notify();
                                                        }))
                                                })
                                                .child(Icon::new(IconName::ChevronRight).small()),
                                        )
                                    })
                                    // 新增标签按钮
                                    .child(
                                        div()
                                            .id("new-tab-btn")
                                            .h(px(40.0))
                                            .w(px(NEW_TAB_BUTTON_WIDTH))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .text_color(theme.muted_foreground)
                                            .hover(|s| s.bg(theme.muted_background).text_color(theme.foreground))
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
                                    .bg(theme.background)
                                    .children([
                                        crate::ui::request::render_url_bar(self, window, cx).into_any_element(),
                                        // 标签页（全部可点击）
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .h(px(crate::ui::components::TAB_H))
                                            .px(px(crate::ui::components::GAP_S))
                                            .border_b(px(1.0))
                                            .border_color(theme.border)
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
                                            BuilderTab::Params => crate::ui::request::render_params_panel(self, window, cx).into_any_element(),
                                            BuilderTab::Authorization => crate::ui::request::render_auth_panel(self, window, cx).into_any_element(),
                                            BuilderTab::Headers => crate::ui::request::render_headers_panel(self, window, cx).into_any_element(),
                                            BuilderTab::Body => crate::ui::request::render_body_panel(self, window, cx).into_any_element(),
                                            BuilderTab::PreRequest => crate::ui::request::render_pre_request_panel(self, window, cx).into_any_element(),
                                            BuilderTab::Tests => crate::ui::request::render_tests_panel(self, window, cx).into_any_element(),
                                            BuilderTab::Settings => crate::ui::request::render_settings_panel(self, cx).into_any_element(),
                                        }
                                    ]),
                                // Splitter（可拖拽调整上下区域大小）
                                div()
                                    .flex_none()
                                    .h(px(6.0))
                                    .w_full()
                                    .bg(theme.muted_background)
                                    .border_t(px(1.0))
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_row_resize()
                                    .hover(|s| s.bg(theme.accent).opacity(0.3))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                        let y: f32 = event.position.y.into();
                                        this.start_splitter_drag(y);
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
                                            .h(px(crate::ui::components::TAB_H))
                                            .px(px(crate::ui::components::GAP_S))
                                            .items_center()
                                            .gap_0()
                                            .border_b(px(1.0))
                                            .border_color(theme.border)
                                            .children([
                                                self.response_tab_button(cx, "response.body", ResponseTab::Body, response_tab, "response-body"),
                                                self.response_tab_button(cx, "response.cookies", ResponseTab::Cookies, response_tab, "response-cookies"),
                                                self.response_tab_button(cx, "response.headers", ResponseTab::Headers, response_tab, "response-headers"),
                                                self.response_tab_button(cx, "response.test_results", ResponseTab::TestResults, response_tab, "response-test-results"),
                                            ]),
                                        // 响应内容区
                                        div()
                                            .flex_1()
                                            .p_3()
                                            .overflow_y_hidden()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .children([
                                                if let Some(err) = error_message {
                                                    let parts: Vec<&str> = err.split("-> ").collect();
                                                    div()
                                                        .w_full()
                                                        .p_3()
                                                        .rounded_md()
                                                        .bg(rgb(0x3f2020))
                                                        .text_color(theme.error)
                                                        .text_xs()
                                                        .overflow_x_hidden()
                                                        .flex_col()
                                                        .gap_1()
                                                        .children(parts.iter().enumerate().map(|(i, part)| {
                                                            let text = if i == 0 { part.to_string() } else { format!("-> {}", part) };
                                                            div().w_full().overflow_x_hidden().text_ellipsis().child(text)
                                                        }))
                                                } else if let Some(resp) = response {
                                                    let theme = self.cached_theme.clone();
                                                    let header_row = div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .justify_between()
                                                        .w_full()
                                                        .mb_3()
                                                        .children([
                                                            // 左侧：状态徽章 + 响应时间 + 响应大小（原本 Time/Size 在右、切换按钮飘在中间）
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_3()
                                                                .children([
                                                                    div()
                                                                        .px_2p5()
                                                                        .py_px()
                                                                        .rounded_md()
                                                                        .bg(if (200..300).contains(&resp.status) { theme.success } else { theme.error })
                                                                        .text_color(theme.accent_foreground)
                                                                        .text_xs()
                                                                        .font_weight(FontWeight(600.0))
                                                                        .child(format!("{} {}", resp.status, resp.status_text())),
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(theme.muted_foreground)
                                                                        .child(format!("{} {}ms", self.t("response.time"), resp.time_ms)),
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(theme.muted_foreground)
                                                                        .child(format!("{} {}", self.t("response.size"), format_size(resp.size_bytes))),
                                                                ])
                                                                .into_any_element(),
                                                            // 右侧：查看模式切换（仅响应体标签页），三个按钮样式完全一致
                                                            if response_tab == ResponseTab::Body {
                                                                segment_group(&theme)
                                                                    .children([
                                                                        segment_button("pretty", self.t("ui.pretty"), self.body_view_mode == BodyViewMode::Pretty, &theme)
                                                                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                this.body_view_mode = BodyViewMode::Pretty;
                                                                                cx.notify();
                                                                            }))
                                                                            .into_any_element(),
                                                                        segment_button("raw", self.t("ui.raw"), self.body_view_mode == BodyViewMode::Raw, &theme)
                                                                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                this.body_view_mode = BodyViewMode::Raw;
                                                                                cx.notify();
                                                                            }))
                                                                            .into_any_element(),
                                                                        segment_button("preview", self.t("ui.preview"), self.body_view_mode == BodyViewMode::Preview, &theme)
                                                                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                                                                this.body_view_mode = BodyViewMode::Preview;
                                                                                cx.notify();
                                                                            }))
                                                                            .into_any_element(),
                                                                    ])
                                                                    .into_any_element()
                                                            } else {
                                                                div().into_any_element()
                                                            },
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
                                                                        // 响应头表格的列标题同样跟着语言切换
                                                                        div().flex_1().text_xs().text_center().text_color(theme.muted_foreground).child(self.t("ui.key")),
                                                                        div().flex_1().text_xs().text_center().text_color(theme.muted_foreground).child(self.t("ui.value")),
                                                                    ])
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex_col()
                                                                    .gap_1()
                                                                    .h(px(300.0))
                                                .overflow_y_scrollbar()
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
                                                                    Input::new(&self.response_pretty_input)
                                                                        .w_full()
                                                                        .h_full()
                                                                        .bg(theme.code_background)
                                                                        .text_color(theme.foreground)
                                                                        .border_0(),
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
                                                                        .h_full()
                                                                        .bg(theme.code_background)
                                                                        .text_color(theme.foreground)
                                                                        .border_0(),
                                                                )
                                                        },
                                                        BodyViewMode::Preview => {
                                                            if let Some(resp) = self.response.as_ref() {
                                                                let ct = resp.detect_content_type();
                                                                let ct_str = ct.as_deref().unwrap_or("").to_lowercase();
                                                                // 光栅图片：原生分辨率 + 双向滚动条
                                                                if ct_str.starts_with("image/") && ct_str != "image/svg+xml" {
                                                                    if let (Some(raw), Some(fmt)) = (resp.raw_body.as_deref(), content_type_to_image_format(&ct_str)) {
                                                                        if let Some(ri) = decode_image_bytes(raw, fmt) {
                                                                            let img_size = ri.size(0);
                                                                            let raw_bytes = raw.to_vec();
                                                                            let ext = match fmt {
                                                                                ImageFormat::Png => "png", ImageFormat::Jpeg => "jpg",
                                                                                ImageFormat::Gif => "gif", ImageFormat::Webp => "webp",
                                                                                ImageFormat::Bmp => "bmp", ImageFormat::Tiff => "tiff",
                                                                                ImageFormat::Ico => "ico", _ => "img",
                                                                            };
                                                                            div()
                                                                                .h_full()
                                                                                .w_full()
                                                                                .flex_col()
                                                                                .bg(theme.code_background)
                                                                                .child(
                                                                                    div()
                                                                                        .h(px(300.0))
                                                                                        .w_full()
                                                                                        .overflow_hidden()
                                                                                        .flex()
                                                                                        .items_center()
                                                                                        .justify_center()
                                                                                        .child(
                                                                                            img(ImageSource::Render(ri))
                                                                                                .h(px(300.0)),
                                                                                        ),
                                                                                )
                                                                                .child(
                                                                                    div()
                                                                                        .w_full()
                                                                                        .flex()
                                                                                        .flex_row()
                                                                                        .items_center()
                                                                                        .justify_between()
                                                                                        .p_2()
                                                                                        .bg(theme.muted_background)
                                                                                        .border_t_1()
                                                                                        .border_color(theme.border)
                                                                                        .children([
                                                                                            div().text_xs().text_color(theme.muted_foreground)
                                                                                                .child(format!("{}×{} px | {}", img_size.width.0, img_size.height.0, ct_str)),
                                                                                            div().cursor_pointer().px_3().py_1().rounded_md()
                                                                                                .bg(theme.accent).text_color(theme.accent_foreground).text_sm()
                                                                                                .child(self.t("preview.open_external"))
                                                                                                .on_mouse_down(MouseButton::Left, {
                                                                                                    let raw_bytes = raw_bytes.clone();
                                                                                                    let ext = ext.to_string();
                                                                                                    move |_event, _window, _cx| {
                                                                                                        let tmp_path = std::env::temp_dir().join(
                                                                                                            format!("apipost-preview-{}.{}", uuid::Uuid::new_v4(), ext));
                                                                                                        if let Err(e) = std::fs::write(&tmp_path, &raw_bytes) {
                                                                                                            log::error!("Failed to write temp image: {}", e);
                                                                                                            return;
                                                                                                        }
                                                                                                        let _ = std::process::Command::new("xdg-open")
                                                                                                            .arg(tmp_path.to_string_lossy().to_string())
                                                                                                            .spawn();
                                                                                                    }
                                                                                                }),
                                                                                        ]),
                                                                                )
                                                                        } else {
                                                                            div().h_full().flex().items_center().justify_center()
                                                                                .text_color(theme.muted_foreground)
                                                                                .child("Image decode failed")
                                                                        }
                                                                    } else {
                                                                        div().h_full().flex().items_center().justify_center()
                                                                            .text_color(theme.muted_foreground)
                                                                            .child("Image response detected")
                                                                    }
                                                                // JSON 和文本类型用 Input 编辑器（自动换行）
                                                                } else if ct_str.contains("json")
                                                                    || ct_str.starts_with("text/plain")
                                                                {
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
                                                                                .h_full()
                                                                                .disabled(true),
                                                                        )
                                                                } else if ct_str.contains("html") {
                                                                    // HTML：上方按钮栏 + 下方源码编辑器（两个独立的 div）
                                                                    let body = resp.body.clone();
                                                                    div()
                                                                        .h_full()
                                                                        .flex_col()
                                                                        .overflow_hidden()
                                                                        .children([
                                                                            // 按钮栏
                                                                            div()
                                                                                .flex()
                                                                                .flex_row()
                                                                                .items_center()
                                                                                .gap_3()
                                                                                .p_2()
                                                                                .children([
                                                                                    div().text_color(theme.muted_foreground).text_sm().child("HTML Response"),
                                                                                    div()
                                                                                        .cursor_pointer()
                                                                                        .px_3()
                                                                                        .py_1()
                                                                                        .rounded_md()
                                                                                        .bg(theme.accent)
                                                                                        .text_color(theme.accent_foreground)
                                                                                        .text_sm()
                                                                                        .child(self.t("preview.open_in_browser"))
                                                                                        .on_mouse_down(MouseButton::Left, {
                                                                                            let body = body.clone();
                                                                                            move |_event, _window, _cx| {
                                                                                                let tmp_path = std::env::temp_dir()
                                                                                                    .join(format!("apipost-preview-{}.html", uuid::Uuid::new_v4()));
                                                                                                if let Err(e) = std::fs::write(&tmp_path, &*body) {
                                                                                                    log::error!("Failed to write temp HTML file: {}", e);
                                                                                                    return;
                                                                                                }
                                                                                                let _ = std::process::Command::new("xdg-open")
                                                                                                    .arg(tmp_path.to_string_lossy().to_string())
                                                                                                    .spawn();
                                                                                            }
                                                                                        }),
                                                                                ]),
                                                                            // 源码编辑器（flex_1 填充剩余空间）
                                                                            div()
                                                                                .h_full()
                                                                                .flex_col()
                                                                                .w_full()
                                                                                .bg(theme.code_background)
                                                                                .border_1()
                                                                                .border_color(theme.border)
                                                                                .rounded_md()
                                                                                .overflow_hidden()
                                                                                .child(
                                                                                    Input::new(&self.response_input)
                                                                                        .w_full()
                                                                                        .h_full()
                                                                                        .disabled(true)
                                                                                ),
                                                                        ])
                                                                } else {
                                                                    // 图片/SVG/PDF 等类型
                                                                    let t = |key: &str| self.t(key);
                                                                    div()
                                                                        .flex_1()
                                                                        .flex_col()
                                                                        .overflow_hidden()
                                                                        .child(render_preview_body(
                                                                            resp.body.as_ref(),
                                                                            ct.as_deref(),
                                                                            &theme,
                                                                            &t,
                                                                            resp.raw_body.as_deref(),
                                                                            self.response_highlight
                                                                                .as_ref()
                                                                                .map(|cache| cache.highlighted.as_ref()),
                                                                        ).into_any_element())
                                                                }
                                                            } else {
                                                                div()
                                                                    .flex_1()
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .text_color(theme.muted_foreground)
                                                                    .child(self.t("preview.not_available"))
                                                            }
                                                        },
                                                        }
                                                    };

                                                    {
                                                        let area = div()
                                                            .flex_1()
                                                            .flex_col()
                                                            .overflow_hidden()
                                                            .child(header_row);
                                                        if response_tab == ResponseTab::Body && self.body_view_mode == BodyViewMode::Pretty {
                                                            let is_json = self.response_raw_format == RawFormat::Json;
                                                            let is_xml = self.response_raw_format == RawFormat::Xml;
                                                            let is_text = self.response_raw_format == RawFormat::Text;
                                                            let is_html = self.response_raw_format == RawFormat::Html;
                                                            area.child(
                                                                // 与请求侧共用同一套分段按钮：原来这里选中/未选中用的是同一个底色（深色主题下 muted==input），
                                                                // 根本看不出选中了哪个格式
                                                                div()
                                                                    .flex()
                                                                    .flex_row()
                                                                    .items_center()
                                                                    .px(px(GAP_S))
                                                                    .child(
                                                                        segment_group(&theme).children([
                                                                            segment_button("resp-fmt-json", "JSON", is_json, &theme)
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(0, _window, cx); }))
                                                                                .into_any_element(),
                                                                            segment_button("resp-fmt-xml", "XML", is_xml, &theme)
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(1, _window, cx); }))
                                                                                .into_any_element(),
                                                                            segment_button("resp-fmt-text", "Text", is_text, &theme)
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(2, _window, cx); }))
                                                                                .into_any_element(),
                                                                            segment_button("resp-fmt-html", "HTML", is_html, &theme)
                                                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| { this.set_response_raw_format(3, _window, cx); }))
                                                                                .into_any_element(),
                                                                        ]),
                                                                    ),
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
                            // 请求发送中的 loading 遮罩（覆盖主工作区）
                            if is_loading {
                                div()
                                    .absolute().top_0().left_0().right_0().bottom_0()
                                    .bg(rgba(0x00000055))
                                    .flex().items_center().justify_center().flex_col().gap_4()
                                    .occlude()
                                    .child(
                                        svg()
                                            .path("icons/loader.svg")
                                            .flex_none()
                                            .size_4()
                                            .text_color(theme.accent)
                                            .with_animation(
                                                ElementId::Name("loading-spinner".into()),
                                                Animation::new(std::time::Duration::from_millis(1200)).repeat(),
                                                |svg, delta| svg.with_transformation(Transformation::rotate(radians(delta * 2.0 * std::f32::consts::PI)))
                                            )
                                    )
                                    .child(div().text_sm().text_color(theme.muted_foreground).child(self.t("ui.sending")))
                            } else {
                                div()
                            },
                            ]),
                    ]),
                // ==================== 底部状态栏 ====================
                // 左：当前环境（+ 代理开启提示）；右：自动保存状态点 + 版本号
                // 之前这里是 "Online / Console / Ready" 和 "*"、"Bearer Token" 之类的占位文字，既不美观也没信息量
                div()
                    .h(px(26.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(crate::ui::components::GAP_M))
                    .bg(theme.muted_background)
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .children([
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(crate::ui::components::GAP_S))
                            .children([
                                div().child(Icon::new(IconName::Globe).xsmall()),
                                div().child(
                                    self.active_environment_name
                                        .clone()
                                        .map(SharedString::from)
                                        .unwrap_or_else(|| self.t("env.no_env"))
                                ),
                                if proxy_on {
                                    div()
                                        .px(px(6.0))
                                        .rounded_sm()
                                        .bg(theme.border)
                                        .text_color(theme.foreground)
                                        .child(self.t("settings.proxy"))
                                } else {
                                    div()
                                },
                            ]),
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(crate::ui::components::GAP_M))
                            .children([
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(crate::ui::components::GAP_XS + 2.0))
                                    .children([
                                        div()
                                            .w(px(6.0))
                                            .h(px(6.0))
                                            .rounded_full()
                                            .bg(if auto_save_on { theme.success } else { theme.border }),
                                        div().child(self.t("settings.auto_save")),
                                    ]),
                                div().child(format!("v{}", env!("CARGO_PKG_VERSION"))),
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
                        &|key| self.t(key),
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
                self.proxy_tips_hovered,
                |d| {
                    let x = self.proxy_tips_x.unwrap_or(0.0);
                    let y = self.proxy_tips_y.unwrap_or(0.0);
                    let tips = self.app_state.lock().unwrap().t("settings.proxy_tips");
                    d.child(
                        tooltip_popup(&theme, x + 16.0, y - 4.0, &tips)
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
                        menu = render_folder_context_menu(&target_id, &folder.name, cx, &theme, &|key| self.t(key))
                            .absolute()
                            .left(px((x - 140.0).max(0.0)))
                            .top(px(y + 4.0));
                    } else if let Some(req) = self.saved_requests.iter().find(|r| r.id == target_id) {
                        menu = render_request_context_menu(req, cx, &theme, &|key| self.t(key))
                            .absolute()
                            .left(px((x - 120.0).max(0.0)))
                            .top(px(y + 4.0));
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
                self.code_gen_dialog_state.lock().map(|s| s.open).unwrap_or(false),
                |d| {
                    let theme_c = theme.clone();
                    let eid = cx.entity_id();
                    d.child(
                        crate::ui::dialogs::render_code_gen_dialog_overlay(
                            &self.code_gen_dialog_state,
                            &theme_c,
                            eid,
                            &|key| self.t(key),
                            window,
                            cx,
                        )
                    )
                },
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
                    let dialog_title: gpui::SharedString = self.t("dialog.save_to_collections").into();
                    let title_label: gpui::SharedString = self.t("dialog.title_label").into();
                    let cancel_text: gpui::SharedString = self.t("dialog.cancel").into();
                    let save_text: gpui::SharedString = self.t("dialog.save").into();
                    d.child(
                        div()
                            .absolute()
                            .top_0().left_0().right_0().bottom_0()
                            .bg(rgba(0x00000055))
                            .flex().items_center().justify_center()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                                                    .child(div().text_sm().font_weight(FontWeight(600.0)).text_color(theme.foreground).child(dialog_title.clone()))
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
                                                        .child(div().text_xs().font_weight(FontWeight(500.0)).text_color(theme.muted_foreground).child(title_label.clone())),
                                                    )
                                                    .child(Input::new(&name_input).h(px(42.0)).w_full().rounded_md().bg(theme.background).text_color(theme.foreground)),
                                            )
                                    )
                                    .child(
                                        div().h(px(48.0)).flex().flex_row().justify_end().items_center().gap_3().px_5()
                                            .border_t_1().border_color(theme.border).bg(theme.muted_background)
                                            .child({
                                                let s = save_dialog.clone();
                                                Button::new("cancel-save-dialog-btn").label(cancel_text.clone())
                                                    .on_click(move |_, _, cx| {
                                                        if let Ok(mut d) = s.lock() { d.visible = false; d.pending_request = None; }
                                                        cx.notify(entity_id);
                                                    })
                                            })
                                            .child({
                                                let s = save_dialog.clone();
                                                let app = save_app.clone();
                                                Button::new("confirm-save-dialog-btn").icon(IconName::Check).label(save_text.clone())
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

