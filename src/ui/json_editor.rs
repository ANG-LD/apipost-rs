//! JSON 编辑器模块
//!
//! 使用 gpui_component 的 code_editor 实现，支持语法高亮和折叠

use crate::ui::body::BodyState;
use crate::ui::themes::Theme;
use gpui::prelude::*;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui::*;

/// 计算行数
pub fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1)
    }
}

/// JSON编辑器组件
pub fn json_editor(
    body_state: &BodyState,
    _line_count: usize,
    error_message: Option<String>,
    theme: &Theme,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let raw_content = &body_state.raw_content;
    let height = body_state.raw_editor_height;
    let bg = theme.background;

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 工具栏 - 只有 Format 按钮
            div()
                .flex()
                .justify_end()
                .items_center()
                .child(
                    gpui_component::button::Button::new("format-json")
                        .label("Format")
                        .xsmall()
                        .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<crate::ui::MainView>| {
                            this.format_json(window, cx);
                            cx.notify();
                        })),
                ),
            // 使用 gpui_component 的 code_editor 实现
            div()
                .flex_1()
                .min_h(px(200.0))
                .child(
                    Input::new(raw_content)
                        .h(px(height))
                        .bg(bg)
                        .bordered(true),
                ),
            // 错误信息
            if let Some(err) = error_message {
                div().text_sm().text_color(rgb(0xef4444)).child(err)
            } else {
                div().hidden()
            },
        ])
}
