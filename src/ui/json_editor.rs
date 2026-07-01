//! JSON 编辑器模块
//!
//! 使用 gpui_component 的 code_editor 实现，支持语法高亮和折叠

use crate::ui::body::BodyState;
use crate::ui::themes::Theme;
use gpui::prelude::*;
use gpui_component::input::{Input, InputState};
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

/// 通用代码/JSON 编辑器视图（仅供内部和 code_gen_dialog 复用）
pub fn code_editor_view(input: &Entity<InputState>, bg: gpui::Rgba) -> impl IntoElement {
    Input::new(input)
        .h_full()
        .bg(bg)
        .bordered(true)
}

/// JSON编辑器组件（不含 Format 按钮，由外部 toolbar 统一管理）
pub fn json_editor(
    body_state: &BodyState,
    _line_count: usize,
    error_message: Option<String>,
    theme: &Theme,
    _t: &dyn Fn(&str) -> String,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let raw_content = &body_state.raw_content;
    let bg = theme.background;

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 编辑器
            div()
                .flex_1()
                .min_h(px(200.0))
                .child(
                    code_editor_view(raw_content, bg),
                ),
            // 错误信息
            if let Some(err) = error_message {
                div().text_sm().text_color(rgb(0xef4444)).child(err)
            } else {
                div().hidden()
            },
        ])
}
