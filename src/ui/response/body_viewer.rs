//! 响应体查看器组件
//!
//! 支持 Pretty/Raw/Preview 三种显示模式。
//! Pretty 模式格式化 JSON，Raw 模式支持 JSON/XML/Text/HTML 格式切换，Preview 模式渲染 HTML。

use crate::app::HttpResponse;
use crate::ui::BodyViewMode;
use crate::ui::RawFormat;
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::{Sizable, StyledExt};
use std::rc::Rc;
use std::sync::Arc;

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

/// 响应体查看器组件
///
/// 渲染响应体的内容区域，包括状态栏、模式切换按钮和编辑器。
pub fn response_body_viewer(
    response: &HttpResponse,
    body_view_mode: BodyViewMode,
    response_raw_format: RawFormat,
    response_input: Entity<InputState>,
    response_xml_input: Entity<InputState>,
    response_text_input: Entity<InputState>,
    response_html_input: Entity<InputState>,
    response_editor_height: f32,
    _response_soft_wrap: bool,
    theme: &Theme,
    on_view_mode_change: Rc<dyn Fn(BodyViewMode, &mut Window, &mut App)>,
    on_raw_format_change: Arc<dyn Fn(RawFormat, &mut Window, &mut App) + 'static>,
) -> impl IntoElement {
    // 状态栏：状态码、模式选择、时间和大小
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
                .bg(if (200..300).contains(&response.status) {
                    theme.success
                } else {
                    theme.error
                })
                .text_color(theme.accent_foreground)
                .child(format!(
                    "{} {}",
                    response.status,
                    response.status_text()
                )),
            // 模式选择按钮
            div()
                .flex()
                .flex_row()
                .gap_2()
                .children([
                    view_mode_button(
                        "Pretty",
                        BodyViewMode::Pretty,
                        body_view_mode,
                        theme,
                        Rc::clone(&on_view_mode_change),
                    ),
                    view_mode_button(
                        "Raw",
                        BodyViewMode::Raw,
                        body_view_mode,
                        theme,
                        Rc::clone(&on_view_mode_change),
                    ),
                    view_mode_button(
                        "Preview",
                        BodyViewMode::Preview,
                        body_view_mode,
                        theme,
                        Rc::clone(&on_view_mode_change),
                    ),
                ]),
            // Time 和 Size 在右边
            div()
                .flex()
                .flex_row()
                .gap_4()
                .children([
                    div()
                        .text_color(theme.muted_foreground)
                        .child(format!("Time: {}ms", response.time_ms)),
                    div()
                        .text_color(theme.muted_foreground)
                        .child(format!("Size: {}", format_size(response.size_bytes))),
                ]),
        ]);

    // 主体内容区域
    let content = match body_view_mode {
        BodyViewMode::Pretty => {
            div()
                .h(px(response_editor_height))
                .flex_col()
                .overflow_hidden()
                .bg(theme.code_background)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .child(Input::new(&response_input).w_full().h_full())
        }
        BodyViewMode::Raw => {
            div()
                .flex_1()
                .flex_col()
                .overflow_hidden()
                .bg(theme.code_background)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .children([
                    // 格式选择器
                    raw_format_selector(response_raw_format, on_raw_format_change.clone(), theme).into_any_element(),
                    // 响应体内容 - 根据格式显示不同的编辑器
                    raw_editor_content(
                        response_raw_format,
                        response_input,
                        response_xml_input,
                        response_text_input,
                        response_html_input,
                        response_editor_height,
                    ).into_any_element(),
                ])
        }
        BodyViewMode::Preview => {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child("Preview mode not implemented")
        }
    };

    div()
        .flex_1()
        .flex_col()
        .overflow_hidden()
        .children([header_row, content])
}

/// 视图模式切换按钮
fn view_mode_button(
    label: &'static str,
    mode: BodyViewMode,
    current_mode: BodyViewMode,
    theme: &Theme,
    on_change: Rc<dyn Fn(BodyViewMode, &mut Window, &mut App)>,
) -> AnyElement {
    let is_active = current_mode == mode;
    Button::new(format!("view-mode-{:?}", mode).to_lowercase())
        .min_w(px(70.0))
        .label(label)
        .small()
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .bg(if is_active {
            theme.accent
        } else {
            theme.input_background
        })
        .text_color(if is_active {
            theme.accent_foreground
        } else {
            theme.muted_foreground
        })
        .on_click(move |_, _window, cx| {
            on_change(mode, _window, cx);
        })
        .into_any_element()
}

/// Raw 格式选择器
fn raw_format_selector(
    current_format: RawFormat,
    on_raw_format_change: Arc<dyn Fn(RawFormat, &mut Window, &mut App) + 'static>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .gap_2()
        .px_2()
        .py_1()
        .bg(theme.muted_background)
        .children([
            raw_format_button("JSON", RawFormat::Json, current_format, on_raw_format_change.clone(), theme),
            raw_format_button("XML", RawFormat::Xml, current_format, on_raw_format_change.clone(), theme),
            raw_format_button("Text", RawFormat::Text, current_format, on_raw_format_change.clone(), theme),
            raw_format_button("HTML", RawFormat::Html, current_format, on_raw_format_change.clone(), theme),
        ])
}

/// Raw 格式切换按钮
fn raw_format_button(
    label: &'static str,
    format: RawFormat,
    current_format: RawFormat,
    on_raw_format_change: Arc<dyn Fn(RawFormat, &mut Window, &mut App) + 'static>,
    theme: &Theme,
) -> impl IntoElement {
    let is_active = current_format == format;
    div()
        .text_sm()
        .cursor_pointer()
        .min_w(px(50.0))
        .px_2()
        .py_px()
        .rounded_sm()
        .bg(if is_active {
            theme.code_background
        } else {
            theme.muted_background
        })
        .text_color(if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
            on_raw_format_change(format, _window, cx);
        })
        .child(label)
}

/// Raw 模式下的编辑器内容区域
fn raw_editor_content(
    raw_format: RawFormat,
    response_input: Entity<InputState>,
    response_xml_input: Entity<InputState>,
    response_text_input: Entity<InputState>,
    response_html_input: Entity<InputState>,
    editor_height: f32,
) -> impl IntoElement {
    div()
        .flex_1()
        .overflow_hidden()
        .children(
            [
                raw_editor_panel(RawFormat::Json, raw_format, response_input, editor_height),
                raw_editor_panel(RawFormat::Xml, raw_format, response_xml_input, editor_height),
                raw_editor_panel(RawFormat::Text, raw_format, response_text_input, editor_height),
                raw_editor_panel(RawFormat::Html, raw_format, response_html_input, editor_height),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
}

/// 单个 Raw 编辑器面板（根据当前格式条件显示）
fn raw_editor_panel(
    format: RawFormat,
    current_format: RawFormat,
    input: Entity<InputState>,
    editor_height: f32,
) -> Option<impl IntoElement> {
    if current_format == format {
        Some(
            div()
                .h(px(editor_height))
                .child(Input::new(&input).w_full().h_full()),
        )
    } else {
        None
    }
}
