//! 交互式 JSON 树形视图
//!
//! 递归渲染 JSON 树，按类型着色。超大 JSON（>500KB）降级为纯文本。

use gpui::*;
use gpui_component::scroll::ScrollableElement;
use serde_json::Value;

use crate::ui::themes::Theme;

/// JSON 树形视图入口
pub fn json_tree_viewer(json_str: &str, theme: &Theme) -> impl IntoElement {
    let parsed = serde_json::from_str::<Value>(json_str);
    let body_too_large = json_str.len() > 500_000;

    div()
        .h_full()
        .w_full()
        .overflow_y_scrollbar()
        .p_3()
        .child(match parsed {
            Ok(value) if !body_too_large => div()
                .text_sm()
                .child(render_json_node(&value, 0, theme)),
            _ => div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(if body_too_large {
                    "Response body too large for tree view (>500KB). Switch to Raw mode."
                } else {
                    "Invalid JSON"
                }),
        })
}

/// 递归渲染 JSON 节点
fn render_json_node(value: &Value, depth: usize, theme: &Theme) -> AnyElement {
    match value {
        Value::Null => span_like(theme.json_null).child("null").into_any_element(),
        Value::Bool(b) => span_like(theme.json_boolean)
            .child(b.to_string())
            .into_any_element(),
        Value::Number(n) => span_like(theme.json_number)
            .child(n.to_string())
            .into_any_element(),
        Value::String(s) => span_like(theme.json_string)
            .child(format!("\"{}\"", escape_json_str(s)))
            .into_any_element(),
        Value::Array(arr) => {
            if arr.is_empty() {
                span_like(theme.json_bracket).child("[]").into_any_element()
            } else {
                let pad = "  ".repeat(depth + 1);
                let close_pad = "  ".repeat(depth);
                let mut children: Vec<AnyElement> = vec![
                    span_like(theme.json_bracket).child("[").into_any_element(),
                ];
                for (i, item) in arr.iter().enumerate() {
                    let comma = if i < arr.len() - 1 { "," } else { "" };
                    children.push(
                        div()
                            .flex()
                            .flex_row()
                            .children([
                                div().flex_none().text_color(theme.muted_foreground).child(pad.clone()).into_any_element(),
                                render_json_node(item, depth + 1, theme),
                                div().flex_none().text_color(theme.json_bracket).child(comma).into_any_element(),
                            ])
                            .into_any_element(),
                    );
                }
                children.push(
                    div()
                        .flex()
                        .flex_row()
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.muted_foreground)
                                .child(close_pad),
                        )
                        .child(span_like(theme.json_bracket).child("]"))
                        .into_any_element(),
                );
                div().flex_col().children(children).into_any_element()
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                span_like(theme.json_bracket).child("{}").into_any_element()
            } else {
                let pad = "  ".repeat(depth + 1);
                let close_pad = "  ".repeat(depth);
                let entries: Vec<(&String, &Value)> = obj.iter().collect();
                let mut children: Vec<AnyElement> = vec![
                    span_like(theme.json_bracket).child("{").into_any_element(),
                ];
                for (i, (key, val)) in entries.iter().enumerate() {
                    let comma = if i < entries.len() - 1 { "," } else { "" };
                    children.push(
                        div()
                            .flex()
                            .flex_row()
                            .children([
                                div().flex_none().text_color(theme.muted_foreground).child(pad.clone()).into_any_element(),
                                span_like(theme.json_key).child(format!("\"{}\"", escape_json_str(key))).into_any_element(),
                                span_like(theme.json_bracket).child(": ").into_any_element(),
                                render_json_node(val, depth + 1, theme),
                                div().flex_none().text_color(theme.json_bracket).child(comma).into_any_element(),
                            ])
                            .into_any_element(),
                    );
                }
                children.push(
                    div()
                        .flex()
                        .flex_row()
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.muted_foreground)
                                .child(close_pad),
                        )
                        .child(span_like(theme.json_bracket).child("}"))
                        .into_any_element(),
                );
                div().flex_col().children(children).into_any_element()
            }
        }
    }
}

/// 创建类似 span 的内联元素（用 flex_none div 模拟）
fn span_like(color: gpui::Rgba) -> Div {
    div().flex_none().text_color(color)
}

/// 转义 JSON 字符串中的特殊字符以便显示
fn escape_json_str(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}
