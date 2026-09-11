use gpui::*;
use gpui_component::StyledExt;

use crate::ui::components::{RADIUS_MD, RADIUS_SM};
use crate::ui::Theme;

/// 上下文菜单面板（返回 Div，可继续链式调用）
pub fn popup_panel(theme: &Theme) -> gpui::Div {
    div()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        // 浮层用卡片档圆角（8px），比控件档大一档，层级才看得出来
        .rounded(px(RADIUS_MD))
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
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        // 提示条面积小，用控件档圆角，和按钮同一套
        .rounded(px(RADIUS_SM))
        .shadow_lg()
        .px_2()
        .py_1()
        .text_sm()
        .whitespace_nowrap()
        .text_color(theme.foreground)
        .child(text.to_string())
}
