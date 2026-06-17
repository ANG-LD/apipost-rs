# Response Viewer: Pretty / Raw / Preview 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复响应面板 Pretty/Raw/Preview 三种显示模式，Pretty 支持语法高亮，Raw 显示原始文本并可切换格式，Preview 根据 Content-Type 智能预览

**Architecture:** 将响应体渲染逻辑从 `main_view.rs:3199-3291` 提取为独立函数，Pretty 模式用 `highlight_json()` 生成彩色 token 渲染，Raw 模式复用现有 entity 架构正确切换内容，Preview 新增 content-type 路由（图片/HTML/JSON树/文本），JSON 树形视图作为新组件

**Tech Stack:** Rust, gpui, gpui_component (Input/Button), serde_json

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/ui/response/json_tree_viewer.rs` | 新建 | 交互式 JSON 树形视图组件 |
| `src/ui/response/mod.rs` | 修改 | 注册新模块 |
| `src/ui/main_view.rs` | 修改 | 响应体渲染逻辑、数据存储、Preview 路由 |
| `src/ui/body.rs` | 修改 | 导出 `highlight_json`（已有，确认可访问） |
| `src/ui/response/body_viewer.rs` | 修改 | 删除未使用的函数 |
| `src/i18n/mod.rs` | 修改 | 添加新的 i18n key |

---

### Task 1: 调整数据存储 — response_input 改为存原始 body

**Files:**
- Modify: `src/ui/main_view.rs:718-721`

- [ ] **Step 1: 修改 response_input 的存储值为原始 body**

将 lines 718-721 从:
```rust
let json_body = RawFormat::Json.format_body(&response.body);
this.response_input.update(cx, |state, cx| {
    state.set_value(&json_body, window, cx);
});
```
改为:
```rust
this.response_input.update(cx, |state, cx| {
    state.set_value(&response.body, window, cx);
});
```

删除 `let json_body = ...` 行。格式化逻辑移入 Pretty 模式渲染时动态执行。

- [ ] **Step 2: 验证编译**

```bash
cargo check 2>&1 | head -30
```
Expected: 编译通过（如果 main_view.rs 中其他地方读取 `response_input` 的值做了 JSON 相关处理，可能需要调整）。

- [ ] **Step 3: 检查并修复 `response_input` 的其他读取点**

搜索所有读取 `self.response_input` 的地方，确认不会因内容从 formatted JSON 变为 raw body 而出问题。

```bash
grep -n 'response_input' src/ui/main_view.rs
```
Expected: 仅 Pretty/Raw 模式渲染处读取。如果 history replay 等处也有读取，需要相应调整。

- [ ] **Step 4: Commit**

```bash
git add src/ui/main_view.rs
git commit -m "fix: store raw body in response_input, move formatting to render layer"
```

---

### Task 2: 实现 Pretty 模式语法高亮渲染

**Files:**
- Modify: `src/ui/main_view.rs:3200-3213`

- [ ] **Step 1: 在 main_view.rs 中添加 pretty_response_viewer 函数**

在文件末尾（`impl MainView` 之外）添加:

```rust
/// Pretty 模式：格式化 + 语法高亮
fn render_pretty_body(body: &str, format: RawFormat, theme: &Theme) -> impl IntoElement {
    let formatted = match format {
        RawFormat::Json => {
            // 尝试格式化 JSON，失败则返回原始文本
            serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                .unwrap_or_else(|| body.to_string())
        }
        _ => body.to_string(),
    };

    let font_family_list: Vec<FontFamily> = vec![
        FontFamily::Name("Menlo".into()),
        FontFamily::Name("Consolas".into()),
        FontFamily::Name("monospace".into()),
    ];

    div()
        .h_full()
        .w_full()
        .overflow_scroll()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .p_3()
        .font_family(font_family_list)
        .text_sm()
        .child(
            div().flex_col().gap_px().children(
                if format == RawFormat::Json {
                    let tokens = crate::ui::body::highlight_json(&formatted, theme);
                    render_highlighted_tokens(&tokens)
                } else {
                    vec![div().text_color(theme.foreground).child(formatted.clone())]
                }
            )
        )
}

/// 将 highlight_json 返回的 token 列表渲染为带颜色的文本行
fn render_highlighted_tokens(tokens: &[(String, gpui::Rgba)]) -> Vec<AnyElement> {
    let mut elements: Vec<AnyElement> = Vec::new();
    let mut line_elements: Vec<AnyElement> = Vec::new();

    for (text, color) in tokens {
        let parts: Vec<&str> = text.split_inclusive('\n').collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                let display_text = if part.ends_with('\n') {
                    &part[..part.len() - 1]
                } else {
                    part
                };
                if !display_text.is_empty() {
                    line_elements.push(
                        span()
                            .text_color(*color)
                            .child(display_text.to_string())
                            .into_any_element()
                    );
                }
            }
            if part.ends_with('\n') || (i < parts.len() - 1) {
                elements.push(
                    div().flex().flex_row().children(std::mem::take(&mut line_elements)).into_any_element()
                );
                line_elements.clear();
            }
        }
    }
    if !line_elements.is_empty() {
        elements.push(
            div().flex().flex_row().children(std::mem::take(&mut line_elements)).into_any_element()
        );
    }
    elements
}
```

- [ ] **Step 2: 替换 Pretty 模式的渲染分支**

将 `main_view.rs:3200-3213` 的:
```rust
BodyViewMode::Pretty => {
    div()
        .h_full()
        .flex_col()
        .overflow_hidden()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            Input::new(&self.response_input)
                .w_full()
                .h_full(),
        )
},
```
替换为:
```rust
BodyViewMode::Pretty => {
    let resp_body = self.response.as_ref().map(|r| r.body.as_str()).unwrap_or("");
    render_pretty_body(resp_body, self.response_raw_format, &theme)
},
```

同时需要在文件顶部添加 `use crate::ui::body::highlight_json;`（如果尚未导入），以及确保 `FontFamily` 已导入。

- [ ] **Step 3: 编译验证**

```bash
cargo check 2>&1 | head -50
```
Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/ui/main_view.rs
git commit -m "feat: add syntax-highlighted Pretty mode rendering"
```

---

### Task 3: 修复 Raw 模式 — 子格式选择器切换内容

**Files:**
- Modify: `src/ui/main_view.rs:3215-3229` (Raw 渲染分支)
- Modify: `src/ui/main_view.rs:3248-3291` (Raw 子格式选择器 + 内容区域组装)

- [ ] **Step 1: 替换 Raw 模式渲染分支，按格式选择 entity**

将 `main_view.rs:3215-3229` 的:
```rust
BodyViewMode::Raw => {
    div()
        .h_full()
        .flex_col()
        .overflow_hidden()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            Input::new(&self.response_input)
                .w_full()
                .h_full(),
        )
},
```
替换为:
```rust
BodyViewMode::Raw => {
    let input_entity = match self.response_raw_format {
        RawFormat::Json => &self.response_input,
        RawFormat::Xml => &self.response_xml_input,
        RawFormat::Text => &self.response_text_input,
        RawFormat::Html => &self.response_html_input,
        _ => &self.response_text_input,
    };
    div()
        .h_full()
        .flex_col()
        .overflow_hidden()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            Input::new(input_entity)
                .w_full()
                .h_full(),
        )
},
```

- [ ] **Step 2: 子格式选择器仅在 Raw + Body tab 显示，逻辑已正确**

当前 `main_view.rs:3248` 的 `if self.body_view_mode == BodyViewMode::Raw` 已经确保只在 Raw 模式下显示子格式选择器。无需修改此条件。

- [ ] **Step 3: 编译验证**

```bash
cargo check 2>&1 | head -50
```
Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/ui/main_view.rs
git commit -m "fix: Raw mode now switches displayed entity based on format selector"
```

---

### Task 4: 添加 i18n keys

**Files:**
- Modify: `src/i18n/mod.rs` (中文和英文翻译表)

- [ ] **Step 1: 在中文和英文翻译表中添加新 key**

在中文翻译 `fn chinese()` 返回的 map 中（约 line 140 附近），添加:
```rust
map.insert("preview.not_available".to_string(), "无法预览此内容类型".to_string());
map.insert("preview.open_in_browser".to_string(), "在浏览器中打开".to_string());
map.insert("preview.view_source".to_string(), "查看源码".to_string());
map.insert("preview.open_external".to_string(), "在外部程序中打开".to_string());
```

在英文翻译 `fn english()` 返回的 map 中（约 line 325 附近），添加:
```rust
map.insert("preview.not_available".to_string(), "Preview not available for this content type".to_string());
map.insert("preview.open_in_browser".to_string(), "Open in Browser".to_string());
map.insert("preview.view_source".to_string(), "View Source".to_string());
map.insert("preview.open_external".to_string(), "Open in External Program".to_string());
```

- [ ] **Step 2: 验证编译**

```bash
cargo check 2>&1 | head -20
```
Expected: 编译通过。

- [ ] **Step 3: Commit**

```bash
git add src/i18n/mod.rs
git commit -m "feat: add i18n keys for preview mode UI text"
```

---

### Task 5: 创建 JSON 树形视图组件

**Files:**
- Create: `src/ui/response/json_tree_viewer.rs`
- Modify: `src/ui/response/mod.rs`

- [ ] **Step 1: 创建 json_tree_viewer.rs**

```rust
//! 交互式 JSON 树形视图
//!
//! 递归渲染可折叠的 JSON 树，点击箭头展开/折叠嵌套对象和数组。

use gpui::*;
use serde_json::Value;

/// JSON 树形视图入口
pub fn json_tree_viewer(json_str: &str, theme: &crate::ui::themes::Theme) -> impl IntoElement {
    let parsed = serde_json::from_str::<Value>(json_str);
    let body_too_large = json_str.len() > 500_000; // 500KB 阈值

    div()
        .h_full()
        .w_full()
        .overflow_scroll()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .p_3()
        .child(match parsed {
            Ok(value) if !body_too_large => {
                div()
                    .font_family(vec![
                        FontFamily::Name("Menlo".into()),
                        FontFamily::Name("Consolas".into()),
                        FontFamily::Name("monospace".into()),
                    ])
                    .text_sm()
                    .child(render_json_node(&value, 0, true, theme))
            }
            _ => {
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(if body_too_large {
                        "Response body too large for tree view (>500KB). Switch to Raw mode."
                    } else {
                        "Invalid JSON"
                    })
            }
        })
}

/// 递归渲染 JSON 节点
fn render_json_node(value: &Value, depth: usize, _expanded: bool, theme: &crate::ui::themes::Theme) -> AnyElement {
    match value {
        Value::Null => span().text_color(theme.json_null).child("null").into_any_element(),
        Value::Bool(b) => span().text_color(theme.json_boolean).child(b.to_string()).into_any_element(),
        Value::Number(n) => span().text_color(theme.json_number).child(n.to_string()).into_any_element(),
        Value::String(s) => span()
            .text_color(theme.json_string)
            .child(format!("\"{}\"", s))
            .into_any_element(),
        Value::Array(arr) => {
            if arr.is_empty() {
                span().text_color(theme.json_bracket).child("[]").into_any_element()
            } else {
                let indent = "  ".repeat(depth + 1);
                let close_indent = "  ".repeat(depth);
                let mut children: Vec<AnyElement> = Vec::new();
                children.push(span().text_color(theme.json_bracket).child("[").into_any_element());
                for (i, item) in arr.iter().enumerate() {
                    children.push(
                        div()
                            .flex()
                            .flex_row()
                            .children([
                                span().text_color(theme.muted_foreground).child(indent.clone()),
                                render_json_node(item, depth + 1, true, theme),
                                if i < arr.len() - 1 {
                                    span().text_color(theme.json_bracket).child(",").into_any_element()
                                } else {
                                    span().into_any_element()
                                },
                            ])
                            .into_any_element(),
                    );
                }
                children.push(
                    span()
                        .text_color(theme.json_bracket)
                        .child(format!("{close_indent}]"))
                        .into_any_element(),
                );
                div().flex_col().children(children).into_any_element()
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                span().text_color(theme.json_bracket).child("{}").into_any_element()
            } else {
                let indent = "  ".repeat(depth + 1);
                let close_indent = "  ".repeat(depth);
                let mut children: Vec<AnyElement> = Vec::new();
                children.push(span().text_color(theme.json_bracket).child("{").into_any_element());
                let entries: Vec<(&String, &Value)> = obj.iter().collect();
                for (i, (key, val)) in entries.iter().enumerate() {
                    children.push(
                        div()
                            .flex()
                            .flex_row()
                            .children([
                                span().text_color(theme.muted_foreground).child(indent.clone()),
                                span().text_color(theme.json_key).child(format!("\"{}\"", key)),
                                span().text_color(theme.json_bracket).child(": "),
                                render_json_node(val, depth + 1, true, theme),
                                if i < entries.len() - 1 {
                                    span().text_color(theme.json_bracket).child(",").into_any_element()
                                } else {
                                    span().into_any_element()
                                },
                            ])
                            .into_any_element(),
                    );
                }
                children.push(
                    span()
                        .text_color(theme.json_bracket)
                        .child(format!("{close_indent}}}"))
                        .into_any_element(),
                );
                div().flex_col().children(children).into_any_element()
            }
        }
    }
}
```

- [ ] **Step 2: 注册新模块**

在 `src/ui/response/mod.rs` 中添加:
```rust
mod json_tree_viewer;
pub use json_tree_viewer::*;
```

- [ ] **Step 3: 编译验证**

```bash
cargo check 2>&1 | head -50
```
Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/ui/response/json_tree_viewer.rs src/ui/response/mod.rs
git commit -m "feat: add interactive JSON tree viewer component"
```

---

### Task 6: 实现 Preview 模式 — Content-Type 路由

**Files:**
- Modify: `src/ui/main_view.rs:3230-3238` (Preview 分支)、文件顶部 imports

- [ ] **Step 1: 在 main_view.rs 中添加 preview 路由函数**

在文件末尾添加:

```rust
/// 根据 Content-Type 智能选择预览渲染方式
fn render_preview_body(
    body: &str,
    content_type: Option<&str>,
    theme: &crate::ui::themes::Theme,
    t: &dyn Fn(&str) -> String, // i18n 翻译函数
) -> AnyElement {
    let ct = content_type.unwrap_or("").to_lowercase();

    // 图片类型 — 使用 gpui img() 渲染
    if ct.starts_with("image/") && ct != "image/svg+xml" {
        // 对于非 SVG 的光栅图片，尝试从 body 字节创建 ImageSource
        // 注意: 我们的 response.body 是 String，图片响应通常是二进制
        // 这里暂时显示提示，因为 String 到 image bytes 的往返可能损坏数据
        return div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .children([
                div().text_color(theme.muted_foreground).child("Image preview not supported for text-encoded responses"),
                div().text_xs().text_color(theme.muted_foreground).child(format!("Content-Type: {}", content_type.unwrap_or("unknown"))),
            ])
            .into_any_element();
    }

    // SVG — 可以作为文本渲染
    if ct == "image/svg+xml" {
        return div()
            .h_full()
            .w_full()
            .overflow_scroll()
            .bg(theme.code_background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .p_3()
            .child(body.to_string())
            .into_any_element();
    }

    // HTML — 提供"在浏览器中打开"按钮，并在下方显示带语法高亮的源码
    if ct == "text/html" {
        let body_owned = body.to_string();
        let body_for_display = body.to_string();
        return div()
            .flex_1()
            .flex_col()
            .overflow_scroll()
            .gap_3()
            .children([
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .children([
                        div().text_color(theme.muted_foreground).text_sm().child("HTML Response"),
                        div()
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(theme.accent)
                            .text_color(theme.accent_foreground)
                            .text_sm()
                            .child(t("preview.open_in_browser"))
                            .on_click({
                                let body = body_owned.clone();
                                move |_event, _window, _cx| {
                                    let tmp_path = std::env::temp_dir().join(format!("apipost-preview-{}.html", uuid::Uuid::new_v4()));
                                    if let Err(e) = std::fs::write(&tmp_path, &body) {
                                        log::error!("Failed to write temp HTML file: {}", e);
                                        return;
                                    }
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg(tmp_path.to_string_lossy().to_string())
                                        .spawn();
                                }
                            }),
                    ]),
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t("preview.view_source")),
                div()
                    .w_full()
                    .overflow_scroll()
                    .bg(theme.code_background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .p_3()
                    .font_family(vec![
                        FontFamily::Name("Menlo".into()),
                        FontFamily::Name("Consolas".into()),
                        FontFamily::Name("monospace".into()),
                    ])
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(body_for_display),
            ])
            .into_any_element();
    }

    // JSON — 树形视图
    if ct.contains("json") {
        return div()
            .h_full()
            .w_full()
            .overflow_scroll()
            .bg(theme.code_background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(crate::ui::response::json_tree_viewer(body, theme))
            .into_any_element();
    }

    // PDF — 在外部程序中打开
    if ct == "application/pdf" {
        let body_owned = body.to_string();
        return div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .children([
                div().text_color(theme.muted_foreground).child("PDF Response"),
                div()
                    .cursor_pointer()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(theme.accent)
                    .text_color(theme.accent_foreground)
                    .text_sm()
                    .child(t("preview.open_external"))
                    .on_click({
                        move |_event, _window, _cx| {
                            let tmp_path = std::env::temp_dir().join(format!("apipost-preview-{}.pdf", uuid::Uuid::new_v4()));
                            if let Err(e) = std::fs::write(&tmp_path, &body_owned) {
                                log::error!("Failed to write temp PDF file: {}", e);
                                return;
                            }
                            let _ = std::process::Command::new("xdg-open")
                                .arg(tmp_path.to_string_lossy().to_string())
                                .spawn();
                        }
                    }),
            ])
            .into_any_element();
    }

    // 文本类型（text/plain, text/css, text/javascript, application/xml...）— 带语法高亮
    let formatted_body = match ct.as_str() {
        _ if ct.contains("json") => {
            serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                .unwrap_or_else(|| body.to_string())
        }
        _ => body.to_string(),
    };

    // 尝试 JSON 语法高亮
    let is_json_like = ct.contains("json") 
        || body.trim_start().starts_with('{') 
        || body.trim_start().starts_with('[');

    div()
        .h_full()
        .w_full()
        .overflow_scroll()
        .bg(theme.code_background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .p_3()
        .font_family(vec![
            FontFamily::Name("Menlo".into()),
            FontFamily::Name("Consolas".into()),
            FontFamily::Name("monospace".into()),
        ])
        .text_sm()
        .child(
            if is_json_like {
                let tokens = crate::ui::body::highlight_json(&formatted_body, theme);
                div().flex_col().children(
                    render_highlighted_tokens(&tokens)
                )
            } else {
                div().text_color(theme.foreground).child(formatted_body)
            }
        )
        .into_any_element()
}
```

- [ ] **Step 2: 替换 Preview 分支**

将 `main_view.rs:3230-3238` 的:
```rust
BodyViewMode::Preview => {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.muted_foreground)
        .child("Preview mode not implemented")
},
```
替换为:
```rust
BodyViewMode::Preview => {
    if let Some(resp) = self.response.as_ref() {
        let ct = resp.detect_content_type();
        let t = |key: &str| self.t(key);
        render_preview_body(
            &resp.body,
            ct.as_deref(),
            &theme,
            &t,
        )
    } else {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child(self.t("preview.not_available"))
    }
},
```

- [ ] **Step 3: 添加必要的 imports**

在文件顶部 `use` 语句区域，确保已导入:
```rust
use crate::ui::response::json_tree_viewer;
use serde_json::Value;
```
`uuid` 应该已有（项目已依赖）。

- [ ] **Step 4: 编译验证**

```bash
cargo check 2>&1 | head -80
```
Expected: 编译通过。

- [ ] **Step 5: Commit**

```bash
git add src/ui/main_view.rs
git commit -m "feat: implement Preview mode with content-type routing"
```

---

### Task 7: 清理 body_viewer.rs 死代码

**Files:**
- Modify: `src/ui/response/body_viewer.rs`
- Modify: `src/ui/response/mod.rs`

- [ ] **Step 1: 删除未使用的函数**

`body_viewer.rs` 中以下函数未被任何代码调用，全部删除：
- `response_body_viewer()` (lines 31-165)
- `view_mode_button()` (lines 168-198)
- `raw_format_selector()` (lines 201-221)
- `raw_format_button()` (lines 224-253)
- `raw_editor_content()` (lines 256-278)
- `raw_editor_panel()` (lines 281-296)

同时可以删除不再需要的 imports（`Rc`, `Arc`, `BodyViewMode`, `RawFormat` 等），因为文件中的 `format_size()` 也不再被调用。保留文件骨架（mod 声明和 license 注释如需要）。

实际做法：清空文件只保留 `//!` 文档注释头部，或直接删除文件。

- [ ] **Step 2: 更新 mod.rs**

如果删除了文件内容，在 `src/ui/response/mod.rs` 中删除 `mod body_viewer;` 和 `pub use body_viewer::*;`（如果文件为空，编译器会警告）。

或者保留空文件以维持 git history。

- [ ] **Step 3: 编译验证**

```bash
cargo check 2>&1 | head -30
```
Expected: 编译通过，无 unused import 警告。

- [ ] **Step 4: Commit**

```bash
git add src/ui/response/
git commit -m "chore: remove unused body_viewer functions"
```

---

### Task 8: 集成测试与手动验证

- [ ] **Step 1: 完整编译**

```bash
cargo build 2>&1 | tail -20
```
Expected: 编译成功，无 warning。

- [ ] **Step 2: 运行现有测试**

```bash
cargo test 2>&1 | tail -20
```
Expected: 全部通过。

- [ ] **Step 3: 检查 clippy**

```bash
cargo clippy 2>&1 | tail -30
```
Expected: 无 critical warning（可能有既存 warning，不计入本任务）。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: final integration verification passes"
```
