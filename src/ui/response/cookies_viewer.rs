//! Cookie 查看器组件
//!
//! 以表格形式展示 HTTP 响应的 Cookie 列表。

use crate::http::Cookie;
use crate::ui::Theme;
use gpui::*;
use gpui_component::StyledExt;

/// Cookie 查看器组件
///
/// 展示 Cookie 列表，包含 Name、Value、Domain、Path 四列。
/// 当没有 Cookie 时显示 "No cookies" 提示。
pub fn response_cookies_viewer(cookies: &[Cookie], theme: &Theme) -> impl IntoElement {
    if cookies.is_empty() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child("No cookies");
    }

    div()
        .flex_1()
        .flex_col()
        .overflow_hidden()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .p_2()
        .children([
            // 表头
            div()
                .flex()
                .flex_row()
                .gap_2()
                .mb_2()
                .children([
                    div()
                        .w(px(100.0))
                        .text_color(theme.json_key)
                        .font_bold()
                        .text_sm()
                        .child("Name"),
                    div()
                        .w(px(150.0))
                        .text_color(theme.json_key)
                        .font_bold()
                        .text_sm()
                        .child("Value"),
                    div()
                        .w(px(80.0))
                        .text_color(theme.json_key)
                        .font_bold()
                        .text_sm()
                        .child("Domain"),
                    div()
                        .w(px(80.0))
                        .text_color(theme.json_key)
                        .font_bold()
                        .text_sm()
                        .child("Path"),
                ]),
            // Cookie 行
            div()
                .flex_col()
                .gap_1()
                .children(
                    cookies
                        .iter()
                        .map(|cookie| {
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .p_1()
                                .bg(theme.muted_background)
                                .rounded_sm()
                                .children([
                                    div()
                                        .w(px(100.0))
                                        .text_color(theme.foreground)
                                        .text_sm()
                                        .child(cookie.name.clone()),
                                    div()
                                        .w(px(150.0))
                                        .text_color(theme.foreground)
                                        .text_sm()
                                        .overflow_x_hidden()
                                        .child(cookie.value.clone()),
                                    div()
                                        .w(px(80.0))
                                        .text_color(theme.muted_foreground)
                                        .text_sm()
                                        .child(cookie.domain.clone().unwrap_or_default()),
                                    div()
                                        .w(px(80.0))
                                        .text_color(theme.muted_foreground)
                                        .text_sm()
                                        .child(cookie.path.clone().unwrap_or_default()),
                                ])
                        })
                        .collect::<Vec<_>>(),
                ),
        ])
}
