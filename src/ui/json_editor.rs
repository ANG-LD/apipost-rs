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
pub fn code_editor_view(
    input: &Entity<InputState>,
    bg: gpui::Rgba,
    fg: gpui::Rgba,
) -> impl IntoElement {
    // 编辑器内部取的是 window.text_style().color，没人设置就是接近黑色的默认值，
    // 在深色底上等于看不见（JSON 靠语法高亮才显得正常，纯文本格式就全黑）。
    div()
        .size_full()
        .text_color(fg)
        // 编辑器自身给统一圆角，和外面的面板/卡片保持同一档
        .child(
            Input::new(input)
                .h_full()
                .bg(bg)
                .bordered(true)
                .rounded(px(crate::ui::components::RADIUS_SM)),
        )
}

/// 编辑器面板：与 JSON 编辑框同高（flex_1 + min_h(200)）。
/// 各 Raw 格式（JSON/XML/Text/HTML）都走这里，避免高度不一致。
pub fn code_editor_pane(
    input: &Entity<InputState>,
    bg: gpui::Rgba,
    fg: gpui::Rgba,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_h(px(200.0))
        .child(code_editor_view(input, bg, fg))
}

/// JSON编辑器组件（不含 Format 按钮，由外部 toolbar 统一管理）
pub fn json_editor(
    body_state: &BodyState,
    _line_count: usize,
    error_message: Option<String>,
    theme: &Theme,
    _t: &dyn Fn(&str) -> SharedString,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let raw_content = &body_state.raw_content;
    // 与响应体编辑器（main_view 里用 code_background）保持同一底色
    let bg = theme.code_background;

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 编辑器（与 XML/Text/HTML 共用同一构建器，高度一致）
            code_editor_pane(raw_content, bg, theme.foreground).into_any_element(),
            // 错误信息
            if let Some(err) = error_message {
                div()
                    .text_sm()
                    .text_color(theme.error)
                    .child(err)
                    .into_any_element()
            } else {
                div().hidden().into_any_element()
            },
        ])
}
