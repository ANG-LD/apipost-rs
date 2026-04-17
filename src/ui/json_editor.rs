//! JSON 编辑器模块
//!
//! 为 Raw -> JSON 模式提供格式化、行号显示、语法高亮功能

use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui::*;

/// JSON 语法高亮颜色
pub struct JsonSyntaxColors {
    pub string: gpui::Hsla,
    pub number: gpui::Hsla,
    pub keyword: gpui::Hsla,
    pub bracket: gpui::Hsla,
    pub property: gpui::Hsla,
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

/// 对 JSON 文本进行语法高亮，返回带颜色的 Element
pub fn highlight_json(text: &str, colors: &JsonSyntaxColors) -> Vec<impl IntoElement> {
    let mut result = Vec::new();

    let mut chars = text.chars().peekable();

    fn push_token(tokens: &mut Vec<impl IntoElement>, token: &str, color: gpui::Hsla) {
        if !token.is_empty() {
            tokens.push(
                div()
                    .text_color(color)
                    .child(token.to_string())
            );
        }
    }

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                // 字符串可能是属性名或值
                let mut s = String::from('"');
                while let Some(&next) = chars.peek() {
                    if next == '"' {
                        s.push(chars.next().unwrap());
                        break;
                    }
                    s.push(chars.next().unwrap());
                }

                // 简单判断：如果后面是 : 就是属性名
                let next_non_space = chars.clone().skip_while(|x| x.is_whitespace()).peek();
                let is_property = next_non_space.map(|x| *x == ':').unwrap_or(false);

                push_token(&mut result, &s, if is_property { colors.property } else { colors.string });
            }
            '0'..='9' | '-' => {
                let mut num = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_numeric() || next == '.' || next == 'e' || next == 'E' || next == '+' || next == '-' {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                push_token(&mut result, &num, colors.number);
            }
            'n' | 't' | 'f' => {
                let mut kw = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_alphabetic() {
                        kw.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                let color = match kw.as_str() {
                    "null" | "true" | "false" => colors.keyword,
                    _ => colors.bracket,
                };
                push_token(&mut result, &kw, color);
            }
            '{' | '}' | '[' | ']' => {
                push_token(&mut result, &c.to_string(), colors.bracket);
            }
            ':' | ',' => {
                push_token(&mut result, &c.to_string(), colors.bracket);
            }
            _ => {
                push_token(&mut result, &c.to_string(), colors.bracket);
            }
        }
    }

    result
}

/// 计算行数
pub fn count_lines(text: &str) -> usize {
    text.lines().count().max(1)
}

/// JSON 编辑器组件
///
/// - body_state: BodyState 引用（包含 raw_content）
/// - line_count: 当前行数
/// - error_message: 错误信息（如果有）
pub fn json_editor(
    body_state: &crate::body::BodyState,
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
                        .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                            this.body_state.format_json(cx);
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
