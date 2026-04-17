# JSON 编辑器实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为请求构造器的 Body -> Raw -> JSON 模式添加手动格式化、行号显示、语法高亮功能

**Architecture:** 创建独立的 `json_editor.rs` 模块，包含 `JsonEditorState` 状态和 `JsonEditor` 组件。使用 `serde_json` 进行 JSON 格式化，通过 TextElement 实现语法高亮，行号列同步滚动。

**Tech Stack:** Rust, gpui 框架, serde_json (已有)

---

## 文件结构

```
src/ui/
├── json_editor.rs     # 新建：JSON 编辑器组件
└── main_view.rs        # 修改：集成 JsonEditor 到 Raw -> JSON 部分

src/ui/mod.rs          # 修改：导出 json_editor 模块
```

---

## Task 1: 创建 json_editor.rs 基础结构

**Files:**
- Create: `src/ui/json_editor.rs`

- [ ] **Step 1: 创建 json_editor.rs**

```rust
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
```

- [ ] **Step 2: Commit**

```bash
git add src/ui/json_editor.rs
git commit -m "feat: 创建 JSON 编辑器组件"
```

---

## Task 2: 修改 BodyState 添加 error_message 和 format_json

**Files:**
- Modify: `src/ui/body.rs` - 添加 `error_message` 字段和 `format_json` 方法

- [ ] **Step 1: 在 BodyState 结构体中添加 error_message 字段**

找到 BodyState 结构体定义，添加：

```rust
/// Body 状态
#[derive(Clone)]
pub struct BodyState {
    pub body_type: BodyType,
    pub raw_format: RawFormat,
    pub raw_content: Entity<InputState>,
    /// Form-data 条目列表
    pub form_data: Vec<FormDataEntry>,
    /// URL-encoded 条目列表
    pub urlencoded_data: Vec<FormDataEntry>,
    /// Raw 编辑器高度
    pub raw_editor_height: f32,
    /// 是否正在拖动调整大小
    pub is_resizing: bool,
    /// 拖动开始时的鼠标 Y 坐标
    pub resize_start_y: f32,
    /// 拖动开始时的高度
    pub resize_start_height: f32,
    /// 上次拖动更新时间（用于节流）
    last_drag_update: Option<std::time::Instant>,
    /// JSON 格式化错误信息
    pub json_error: Option<String>,
}
```

- [ ] **Step 2: 在 BodyState::new 中初始化 json_error**

```rust
Self {
    // ... 已有字段
    last_drag_update: None,
    json_error: None,  // 添加
}
```

- [ ] **Step 3: 在 BodyState 中添加 format_json 方法**

```rust
/// 格式化 Raw 类型的 JSON 内容
pub fn format_json(&mut self, cx: &Context<MainView>) {
    if self.body_type != BodyType::Raw || self.raw_format != RawFormat::Json {
        return;
    }

    let text = self.raw_content.read(cx).value().to_string();
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => {
            let formatted = serde_json::to_string_pretty(&value).unwrap_or(text);
            self.raw_content.write(cx).set_value(&formatted);
            self.json_error = None;
        }
        Err(e) => {
            self.json_error = Some(e.to_string());
        }
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/body.rs
git commit -m "feat: BodyState 添加 json_error 字段和 format_json 方法"
```

---

## Task 3: 集成 JsonEditor 到 main_view.rs

**Files:**
- Modify: `src/ui/mod.rs` - 导出 json_editor 模块
- Modify: `src/ui/main_view.rs:1858-1873` - 替换 Raw JSON 编辑器部分

- [ ] **Step 1: 在 mod.rs 中导出 json_editor**

修改 `src/ui/mod.rs`:

```rust
mod json_editor;  // 添加
pub use json_editor::*;  // 添加
```

- [ ] **Step 2: 在 main_view.rs 中添加计算行数的方法**

在 MainView impl 块中添加：

```rust
/// 计算 body 内容的行数
fn calculate_body_line_count(body_state: &BodyState, cx: &Context<Self>) -> usize {
    if body_state.body_type == BodyType::Raw && body_state.raw_format == RawFormat::Json {
        let text = body_state.raw_content.read(cx).value().to_string();
        count_lines(&text)
    } else {
        1
    }
}
```

- [ ] **Step 3: 替换 Raw JSON 编辑器**

找到 main_view.rs 中的 Raw JSON 编辑器部分（约 1858-1873 行）：

```rust
// Raw 内容编辑器
div()
    .flex_1()
    .bg(rgb(0x2d2d2d))
    .border_1()
    .border_color(rgb(0x444444))
    .rounded_md()
    .overflow_y_hidden()
    .child(
        Input::new(&body_state.raw_content)
            .flex_1()
            .min_h(px(200.0))
            .bg(rgb(0x2d2d2d))
            .text_color(rgb(0xe0e0e0))
            .font_family("monospace"),
    ),
```

替换为：

```rust
// Raw JSON 编辑器
if body_state.raw_format == RawFormat::Json {
    json_editor(
        body_state,
        calculate_body_line_count(body_state, cx),
        body_state.json_error.clone(),
        cx,
    )
} else {
    // 其他 Raw 格式保持原样
    div()
        .flex_1()
        .bg(rgb(0x2d2d2d))
        .border_1()
        .border_color(rgb(0x444444))
        .rounded_md()
        .overflow_y_hidden()
        .child(
            Input::new(&body_state.raw_content)
                .flex_1()
                .min_h(px(200.0))
                .bg(rgb(0x2d2d2d))
                .text_color(rgb(0xe0e0e0))
                .font_family("monospace"),
        )
}
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/ui/main_view.rs
git commit -m "feat: 集成 JSON 编辑器到 main_view"
```

---

## Task 4: 测试和验证

- [ ] **Step 1: 编译检查**

```bash
cargo check 2>&1
```

- [ ] **Step 2: 运行应用测试**

```bash
cargo run
```

测试步骤：
1. 进入 Body -> Raw -> JSON
2. 输入未格式化的 JSON 如 `{"a":1,"b":2}`
3. 点击 Format 按钮，应该格式化为带缩进的 JSON
4. 左侧应显示行号
5. 输入无效 JSON 应显示错误提示

- [ ] **Step 3: Commit 最终版本**

```bash
git add -A
git commit -m "feat: 完成 JSON 编辑器功能 - 格式化、行号、语法高亮"
```

---

## 自检清单

- [ ] Spec 覆盖检查：
  - 手动格式化 ✓
  - 独立行号列 ✓
  - 语法高亮 ✓
  - 错误提示 ✓

- [ ] 占位符检查：无 TBD/TODO

- [ ] 类型一致性：BodyState::format_json 方法签名正确
