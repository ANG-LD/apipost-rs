use gpui::*;
use gpui_component::StyledExt;

use crate::ui::Theme;

/// 上下文菜单面板（返回 Div，可继续链式调用）
pub fn popup_panel(theme: &Theme) -> gpui::Div {
    div()
        .bg(rgb(0x1e1e2e))
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .shadow_lg()
        .flex_col()
        .py_1()
        .min_w(px(120.0))
}

/// 悬浮提示
pub fn tooltip_popup(theme: &Theme, x: f32, y: f32, text: &str) -> impl IntoElement {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .bg(rgb(0x1e1e2e))
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .shadow_lg()
        .px_2()
        .py_1()
        .text_sm()
        .whitespace_nowrap()
        .text_color(rgb(0xffffff))
        .child(text.to_string())
}
