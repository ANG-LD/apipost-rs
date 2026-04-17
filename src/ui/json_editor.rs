//! JSON 编辑器模块
//!
//! 为 Raw -> JSON 模式提供格式化、行号显示、语法高亮、折叠功能

use crate::ui::body::BodyState;
use gpui::prelude::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui::*;

/// JSON 语法高亮颜色 - 配色方案
pub struct JsonSyntaxColors {
    pub property: Hsla,   // 属性名 - 浅蓝
    pub string: Hsla,     // 字符串值 - 绿色
    pub number: Hsla,     // 数字 - 橙色
    pub boolean: Hsla,    // 布尔值 - 紫红
    pub null: Hsla,       // null - 红色
    pub bracket: Hsla,    // 括号 - 白色
    pub colon: Hsla,      // 冒号 - 灰色
    pub comma: Hsla,      // 逗号 - 灰色
}

impl Default for JsonSyntaxColors {
    fn default() -> Self {
        Self {
            property: hsla(0.55, 0.8, 0.65, 1.0), // 浅蓝色 #38bdf8
            string: hsla(0.35, 0.7, 0.5, 1.0),    // 绿色 #22c55e
            number: hsla(0.08, 0.9, 0.55, 1.0),   // 橙色 #f97316
            boolean: hsla(0.75, 0.7, 0.6, 1.0),  // 紫红色 #c084fc
            null: hsla(0.0, 0.85, 0.55, 1.0),     // 红色 #ef4444
            bracket: hsla(0.0, 0.0, 0.9, 1.0),    // 白色 #e5e5e5
            colon: hsla(0.0, 0.0, 0.5, 1.0),      // 灰色 #808080
            comma: hsla(0.0, 0.0, 0.5, 1.0),      // 灰色 #808080
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

/// JSON节点类型
#[derive(Debug, Clone)]
pub enum JsonNode {
    Object {
        start: usize,
        end: usize,
        children: Vec<(String, JsonNode)>,
        collapsed: bool,
    },
    Array {
        start: usize,
        end: usize,
        items: Vec<JsonNode>,
        collapsed: bool,
    },
    String(String),
    Number(String),
    Boolean(String),
    Null,
}

/// 解析JSON为树结构
fn parse_json(text: &str) -> Result<JsonNode, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Empty JSON".to_string());
    }
    parse_value(text, 0).map(|(node, _)| node)
}

fn parse_value(text: &str, pos: usize) -> Result<(JsonNode, usize), String> {
    let text = text.trim_start();
    let start_pos = pos + text.len() - text.trim_start().len();

    match text.chars().next() {
        Some('{') => parse_object(text, start_pos),
        Some('[') => parse_array(text, start_pos),
        Some('"') => parse_string(text).map(|(s, end)| (JsonNode::String(s), end)),
        Some('t') | Some('f') => parse_boolean(text),
        Some('n') => parse_null(text),
        Some(c) if c.is_ascii_digit() || c == '-' => parse_number(text),
        _ => Err(format!("Invalid JSON at position {}", pos)),
    }
}

fn parse_object(text: &str, start: usize) -> Result<(JsonNode, usize), String> {
    let mut text = text.trim_start();
    assert_eq!(text.chars().next(), Some('{'));
    text = &text[1..];

    let mut children = Vec::new();
    text = text.trim_start();
    if text.starts_with('}') {
        return Ok((JsonNode::Object { start, end: start + text.len() + 2, children, collapsed: false }, start + text.len() + 2));
    }

    loop {
        text = text.trim_start();
        let (key, key_end) = parse_string(text)?;
        let key = key.trim_matches('"').to_string();
        text = &text[key_end..];
        text = text.trim_start();
        if !text.starts_with(':') {
            return Err("Expected ':' in object".to_string());
        }
        text = &text[1..];
        let (value, value_end) = parse_value(text, start + text.len() - text.len())?;
        text = &text[value_end..];
        children.push((key, value));
        text = text.trim_start();
        if text.starts_with(',') {
            text = &text[1..];
        } else if text.starts_with('}') {
            text = &text[1..];
            break;
        } else {
            return Err("Expected ',' or '}' in object".to_string());
        }
    }
    Ok((JsonNode::Object { start, end: start + (text.len() + 1), children, collapsed: false }, start + text.len() + 1))
}

fn parse_array(text: &str, start: usize) -> Result<(JsonNode, usize), String> {
    let mut text = text.trim_start();
    assert_eq!(text.chars().next(), Some('['));
    text = &text[1..];

    let mut items = Vec::new();
    text = text.trim_start();
    if text.starts_with(']') {
        return Ok((JsonNode::Array { start, end: start + text.len() + 2, items, collapsed: false }, start + text.len() + 2));
    }

    loop {
        text = text.trim_start();
        let (value, value_end) = parse_value(text, start + text.len() - text.len())?;
        text = &text[value_end..];
        items.push(value);
        text = text.trim_start();
        if text.starts_with(',') {
            text = &text[1..];
        } else if text.starts_with(']') {
            text = &text[1..];
            break;
        } else {
            return Err("Expected ',' or ']' in array".to_string());
        }
    }
    Ok((JsonNode::Array { start, end: start + text.len() + 1, items, collapsed: false }, start + text.len() + 1))
}

fn parse_string(text: &str) -> Result<(String, usize), String> {
    let mut result = String::new();
    let mut escaped = false;
    let mut end_idx = 0;

    for (i, c) in text.char_indices() {
        end_idx = i;
        if escaped {
            match c {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                '/' => result.push('/'),
                'b' => result.push('\u{0008}'),
                'f' => result.push('\u{000C}'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                'u' => {
                    if i + 4 < text.len() {
                        let hex = &text[i+1..i+5];
                        if let Ok(code) = u16::from_str_radix(hex, 16) {
                            if let Some(ch) = char::from_u32(code as u32) {
                                result.push(ch);
                            }
                        }
                    }
                }
                _ => result.push(c),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            end_idx = i + 1;
            break;
        } else {
            result.push(c);
        }
    }
    Ok((result, end_idx))
}

fn parse_number(text: &str) -> Result<(JsonNode, usize), String> {
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    let number = text[..end].to_string();
    Ok((JsonNode::Number(number), end))
}

fn parse_boolean(text: &str) -> Result<(JsonNode, usize), String> {
    if text.starts_with("true") {
        Ok((JsonNode::Boolean("true".to_string()), 4))
    } else if text.starts_with("false") {
        Ok((JsonNode::Boolean("false".to_string()), 5))
    } else {
        Err("Invalid boolean".to_string())
    }
}

fn parse_null(text: &str) -> Result<(JsonNode, usize), String> {
    if text.starts_with("null") {
        Ok((JsonNode::Null, 4))
    } else {
        Err("Invalid null".to_string())
    }
}

/// JSON编辑器组件
pub fn json_editor(
    body_state: &BodyState,
    line_count: usize,
    error_message: Option<String>,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let colors = JsonSyntaxColors::default();
    let content = body_state.raw_content.read(cx).value().to_string();
    let json_result = parse_json(&content);

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            // 工具栏
            div()
                .flex()
                .justify_between()
                .items_center()
                .children([
                    div().flex().gap_2().children([
                        Button::new("expand-all").label("展开").xsmall(),
                        Button::new("collapse-all").label("折叠").xsmall(),
                    ]),
                    div().flex().items_center().child(
                        Button::new("format-json")
                            .label("Format")
                            .xsmall()
                            .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<crate::ui::MainView>| {
                                this.format_json(window, cx);
                                cx.notify();
                            })),
                    ),
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
                .overflow_hidden()
                .children([
                    // 行号列
                    div()
                        .w(px(40.0))
                        .h(px(1000.0))
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
                    // 内容区域
                    div()
                        .flex_1()
                        .overflow_y_hidden()
                        .p_2()
                        .font_family("monospace")
                        .text_sm()
                        .text_color(rgb(0xe0e0e0))
                        .child({
                            if content.is_empty() {
                                div().text_color(hsla(0.0, 0.0, 0.4, 1.0)).child("Enter JSON here...")
                            } else {
                                match &json_result {
                                    Ok(node) => div().children(render_json_node(node, &colors, 0, cx)),
                                    Err(_) => div().child(content.clone()),
                                }
                            }
                        }),
                ]),
            // 错误信息
            if let Some(err) = error_message {
                div().text_sm().text_color(rgb(0xef4444)).child(err)
            } else {
                div().hidden()
            },
        ])
}

/// 渲染JSON节点
fn render_json_node(
    node: &JsonNode,
    colors: &JsonSyntaxColors,
    indent: usize,
    _cx: &mut Context<crate::ui::MainView>,
) -> Vec<impl IntoElement> {
    let mut elements = Vec::new();
    let indent_str = "  ".repeat(indent);

    match node {
        JsonNode::Object { children, collapsed, .. } => {
            if children.is_empty() {
                elements.push(div().text_color(colors.bracket).child("{}"));
            } else if *collapsed {
                let summary = if children.len() == 1 {
                    "{ 1 item }".to_string()
                } else {
                    format!("{{ {} items }}", children.len())
                };
                elements.push(div().flex().items_center().children([
                    div().text_color(colors.bracket).child("{"),
                    div().text_color(colors.colon).child(" ... "),
                    div().text_color(colors.bracket).child("}"),
                    div().text_xs().ml_1().text_color(hsla(0.6, 0.8, 0.5, 0.7)).child(summary),
                ]));
            } else {
                elements.push(div().text_color(colors.bracket).child("{"));
                for (key, value) in children {
                    let key_display = format!("\"{}\"", key);
                    elements.push(div().pl_4().flex().items_center().children([
                        div().flex().items_center().children([
                            div().text_color(colors.property).child(key_display),
                            div().text_color(colors.colon).child(": "),
                        ]),
                        div().flex_1().children(render_json_node(value, colors, indent + 1, _cx)),
                    ]));
                }
                elements.push(div().text_color(colors.bracket).child(format!("{}}}", indent_str)));
            }
        }
        JsonNode::Array { items, collapsed, .. } => {
            if items.is_empty() {
                elements.push(div().text_color(colors.bracket).child("[]"));
            } else if *collapsed {
                elements.push(div().flex().items_center().children([
                    div().text_color(colors.bracket).child("["),
                    div().text_color(colors.colon).child(" ... "),
                    div().text_color(colors.bracket).child("]"),
                    div().text_xs().ml_1().text_color(hsla(0.6, 0.8, 0.5, 0.7)).child(format!("{} items", items.len())),
                ]));
            } else {
                elements.push(div().text_color(colors.bracket).child("["));
                for item in items {
                    elements.push(div().pl_4().flex().items_center().children([
                        div().flex_1().children(render_json_node(item, colors, indent + 1, _cx)),
                    ]));
                }
                elements.push(div().text_color(colors.bracket).child(format!("{}]", indent_str)));
            }
        }
        JsonNode::String(s) => {
            elements.push(div().text_color(colors.string).child(format!("\"{}\"", s)));
        }
        JsonNode::Number(n) => {
            elements.push(div().text_color(colors.number).child(n.clone()));
        }
        JsonNode::Boolean(b) => {
            elements.push(div().text_color(colors.boolean).child(b.clone()));
        }
        JsonNode::Null => {
            elements.push(div().text_color(colors.null).child("null"));
        }
    }
    elements
}
