//! 响应头查看器组件
//!
//! 以只读输入框形式展示 HTTP 响应头列表。

use crate::app::HttpResponse;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::StyledExt;

/// 响应头查看器组件
///
/// 将响应头以 "Key: Value" 格式每行一个展示在只读多行输入框中。
pub fn response_headers_viewer(
    response: &HttpResponse,
    editor_height: f32,
    window: &mut Window,
    cx: &mut App,
    theme: &Theme,
) -> impl IntoElement {
    let headers_text = response
        .headers
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let headers_input = cx.new(|cx| {
        InputState::new(window, cx)
            .default_value(&headers_text)
            .multi_line(true)
    });

    div()
        .h(px(editor_height))
        .flex_col()
        .overflow_hidden()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(Input::new(&headers_input).w_full().h_full())
}
