//! JSON 编辑器模块
//!
//! 为 Raw -> JSON 模式提供格式化、行号显示、语法高亮功能

use crate::ui::body::BodyState;
use gpui::prelude::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui::*;
use std::ops::Range;

/// JSON 语法高亮颜色
pub struct JsonSyntaxColors {
    pub string: Hsla,
    pub number: Hsla,
    pub keyword: Hsla,
    pub bracket: Hsla,
    pub property: Hsla,
}

impl Default for JsonSyntaxColors {
    fn default() -> Self {
        Self {
            string: hsla(0.35, 0.7, 0.5, 1.0),    // 绿色
            number: hsla(0.08, 0.9, 0.55, 1.0),   // 橙色
            keyword: hsla(0.6, 0.8, 0.7, 1.0),    // 蓝色
            bracket: hsla(0.0, 0.0, 1.0, 1.0),    // 白色
            property: hsla(0.55, 0.8, 0.65, 1.0), // 浅蓝色
        }
    }
}

/// 计算行数
pub fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1)
    }
}

/// JSON token类型
#[derive(Debug, Clone)]
enum JsonToken {
    Property(String),
    String(String),
    Number(String),
    Keyword(String),
    Bracket(char),
    Colon,
    Comma,
    Whitespace(String),
    Other(String),
}

/// 简单的JSON语法高亮解析器
fn tokenize_json(text: &str) -> Vec<(JsonToken, usize, usize)> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut in_string = false;
    let mut string_start = 0;

    while let Some((i, c)) = chars.next() {
        if in_string {
            let mut s = String::from('"');
            let mut j = i + 1;
            while let Some((idx, ch)) = chars.next() {
                s.push(ch);
                j = idx + ch.len_utf8();
                if ch == '"' {
                    break;
                }
                if ch == '\\' {
                    if let Some((idx2, ch2)) = chars.next() {
                        s.push(ch2);
                        j = idx2 + ch2.len_utf8();
                    }
                }
            }
            tokens.push((JsonToken::String(s), string_start, j));
            in_string = false;
            continue;
        }

        match c {
            '"' => {
                string_start = i;
                in_string = true;
            }
            '0'..='9' | '-' => {
                let mut num = String::from(c);
                while let Some(&(_, next)) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' || next == 'e' || next == 'E' || next == '+' || next == '-' {
                        num.push(chars.next().unwrap().1);
                    } else {
                        break;
                    }
                }
                let len = num.len();
                tokens.push((JsonToken::Number(num), i, i + len));
            }
            'n' | 't' | 'f' => {
                let mut kw = String::from(c);
                while let Some(&(_, next)) = chars.peek() {
                    if next.is_alphabetic() {
                        kw.push(chars.next().unwrap().1);
                    } else {
                        break;
                    }
                }
                let len = kw.len();
                let token = match kw.as_str() {
                    "null" => JsonToken::Keyword(kw),
                    "true" | "false" => JsonToken::Keyword(kw),
                    _ => JsonToken::Other(kw),
                };
                tokens.push((token, i, i + len));
            }
            '{' | '}' | '[' | ']' => {
                tokens.push((JsonToken::Bracket(c), i, i + 1));
            }
            ':' => {
                tokens.push((JsonToken::Colon, i, i + 1));
            }
            ',' => {
                tokens.push((JsonToken::Comma, i, i + 1));
            }
            ' ' | '\t' | '\n' | '\r' => {
                let mut ws = String::from(c);
                while let Some(&(_, next)) = chars.peek() {
                    if next == ' ' || next == '\t' || next == '\n' || next == '\r' {
                        ws.push(chars.next().unwrap().1);
                    } else {
                        break;
                    }
                }
                let len = ws.len();
                tokens.push((JsonToken::Whitespace(ws), i, i + len));
            }
            _ => {
                let mut other = String::from(c);
                while let Some(&(_, next)) = chars.peek() {
                    if !"\"{}[]:,0-9ntf ".contains(next) {
                        other.push(chars.next().unwrap().1);
                    } else {
                        break;
                    }
                }
                let len = other.len();
                tokens.push((JsonToken::Other(other), i, i + len));
            }
        }
    }

    // 第二次遍历，将字符串类型的值区分为属性名和字符串值
    let mut result = Vec::new();
    let mut prev_was_colon = false;
    for (token, start, end) in tokens {
        let is_string_before_colon = matches!(&token, JsonToken::String(_)) && !prev_was_colon;
        let new_token = if is_string_before_colon {
            if let JsonToken::String(s) = token {
                JsonToken::Property(s)
            } else {
                token
            }
        } else {
            token
        };
        prev_was_colon = matches!(&new_token, JsonToken::Colon);
        result.push((new_token, start, end));
    }

    result
}

/// JSON 编辑器组件
pub fn json_editor(
    body_state: &BodyState,
    line_count: usize,
    error_message: Option<String>,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let colors = JsonSyntaxColors::default();
    let content = body_state.raw_content.read(cx).value().to_string();
    let tokens = tokenize_json(&content);

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 工具栏 - 按钮靠右
            div()
                .flex()
                .justify_end()
                .items_center()
                .children([
                    Button::new("format-json")
                        .label("Format")
                        .xsmall()
                        .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<crate::ui::MainView>| {
                            this.format_json(window, cx);
                            cx.notify();
                        })),
                ]),
            // 编辑器主体
            div()
                .flex()
                .flex_1()
                .min_h(px(200.0))
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
                    // 编辑/显示区域
                    div()
                        .flex_1()
                        .h_full()
                        .overflow_y_hidden()
                        .p_2()
                        .children([
                            // 高亮文本显示层
                            div()
                                .flex_1()
                                .h_full()
                                .overflow_y_hidden()
                                .font_family("monospace")
                                .text_sm()
                                .text_color(rgb(0xe0e0e0))
                                .children(tokens.iter().map(|(token, _, _)| {
                                    let (text, color) = match token {
                                        JsonToken::Property(s) => (s.clone(), colors.property),
                                        JsonToken::String(s) => (s.clone(), colors.string),
                                        JsonToken::Number(s) => (s.clone(), colors.number),
                                        JsonToken::Keyword(s) => (s.clone(), colors.keyword),
                                        JsonToken::Bracket(c) => (c.to_string(), colors.bracket),
                                        JsonToken::Colon => (":".to_string(), colors.bracket),
                                        JsonToken::Comma => (",".to_string(), colors.bracket),
                                        JsonToken::Whitespace(s) => (s.clone(), colors.bracket),
                                        JsonToken::Other(s) => (s.clone(), colors.bracket),
                                    };
                                    div()
                                        .text_color(color)
                                        .child(text)
                                })),
                            // 输入层 - 透明文本
                            div()
                                .absolute()
                                .inset_0()
                                .overflow_y_hidden()
                                .p_2()
                                .child(
                                    Input::new(&body_state.raw_content)
                                        .w_full()
                                        .h_full()
                                        .bg(hsla(0.0, 0.0, 0.0, 0.0))
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.0))
                                        .font_family("monospace")
                                        .text_sm(),
                                ),
                        ]),
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
