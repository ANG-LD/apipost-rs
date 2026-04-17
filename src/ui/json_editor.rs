//! JSON 编辑器模块
//!
//! 为 Raw -> JSON 模式提供格式化、行号显示、语法高亮功能

use crate::ui::body::BodyState;
use gpui::prelude::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui::*;

/// JSON 语法高亮颜色
pub struct JsonSyntaxColors {
    pub string: gpui::Rgba,
    pub number: gpui::Rgba,
    pub keyword: gpui::Rgba,
    pub bracket: gpui::Rgba,
    pub property: gpui::Rgba,
}

impl Default for JsonSyntaxColors {
    fn default() -> Self {
        Self {
            string: gpui::rgb(0x22c55e),      // 绿色
            number: gpui::rgb(0xf97316),      // 橙色
            keyword: gpui::rgb(0x3b82f6),     // 蓝色
            bracket: gpui::rgb(0xffffff),     // 白色
            property: gpui::rgb(0x38bdf8),    // 浅蓝色
        }
    }
}

/// 计算行数
pub fn count_lines(text: &str) -> usize {
    // 按换行符分割并计算行数，空行也算一行
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1)
    }
}

/// JSON 编辑器组件
///
/// - body_state: BodyState 引用（包含 raw_content）
/// - line_count: 当前行数
/// - error_message: 错误信息（如果有）
pub fn json_editor(
    body_state: &BodyState,
    line_count: usize,
    error_message: Option<String>,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let colors = JsonSyntaxColors::default();
    let content = body_state.raw_content.clone();

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 工具栏
            div()
                .flex()
                .items_center()
                .gap_2()
                .children([
                    Button::new("format-json")
                        .label("Format")
                        .small()
                        .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<crate::ui::MainView>| {
                            this.format_json(window, cx);
                            cx.notify();
                        })),
                ]),
            // 编辑器主体
            div()
                .flex()
                .flex_1()
                .bg(rgb(0x2d2d2d))
                .border_1()
                .border_color(rgb(0x444444))
                .rounded_md()
                .overflow_y_hidden()
                .children([
                    // 行号列
                    div()
                        .w(px(40.0))
                        .h_full()
                        .bg(rgb(0x252525))
                        .flex_col()
                        .overflow_y_hidden()
                        .border_r(px(1.0))
                        .border_color(rgb(0x333333))
                        .p_1()
                        .children((1..=line_count).map(|i| {
                            div()
                                .text_xs()
                                .text_color(rgb(0x666666))
                                .font_family("monospace")
                                .text_right()
                                .pr_1()
                                .child(i.to_string())
                        })),
                    // 编辑区域
                    div()
                        .flex_1()
                        .h_full()
                        .overflow_y_hidden()
                        .p_2()
                        .child(
                            Input::new(&content)
                                .flex_1()
                                .h_full()
                                .bg(rgb(0x2d2d2d))
                                .text_color(rgb(0xe0e0e0))
                                .font_family("monospace")
                        ),
                ]),
            // 错误信息
            if let Some(err) = error_message {
                div()
                    .text_sm()
                    .text_color(rgb(0xef4444))
                    .child(err)
            } else {
                div().hidden()
            },
        ])
}
