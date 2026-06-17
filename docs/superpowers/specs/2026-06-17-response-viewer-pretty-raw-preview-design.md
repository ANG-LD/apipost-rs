# Response Viewer: Pretty / Raw / Preview 修复

Date: 2026-06-17

## 问题

当前响应面板的 Pretty、Raw、Preview 三种模式行为不正确：

| 模式 | 当前 | 期望 |
|------|------|------|
| Pretty | 格式化 JSON，但无语法高亮（文本全白色） | 格式化 + 按 token 类型的彩色语法高亮 |
| Raw | 显示和 Pretty 相同的格式化 JSON，子格式选择器不切换内容 | 显示原始未格式化文本，子格式选择器正确切换 |
| Preview | 占位文字 "not implemented" | 根据 Content-Type 智能预览（图片/HTML/JSON树/文本） |

## 架构

全部修改集中在 `src/ui/main_view.rs` 的响应体渲染区域（约 lines 3199-3291），替换内联的 match 分支。Preview 的 JSON 树形视图组件新建在 `src/ui/response/json_tree_viewer.rs`。

### 数据流（不动）

响应到达 → `main_view.rs:700-730` 的流程不变：
- `response_input` ← 格式化 JSON
- `response_xml_input` / `response_text_input` / `response_html_input` ← 原始 body
- `response_raw_format` ← 自动检测的格式

### Pretty 模式

- 根据 `response_raw_format` 选择对应的格式化逻辑
- JSON：调用现有的 `highlight_json()`（`body.rs:806`）生成 `Vec<(String, Rgba)>` token 列表，每个 token 渲染为一个带颜色的 `span()`
- XML/HTML：直接做缩进格式化（或保持原样），显示在带代码背景的 scrollable div 中
- Text：不做格式化，直接显示
- 使用 `font_family("Menlo, Consolas, monospace")` 等宽字体
- 整个区域 `overflow_scroll()` 可滚动

Pretty 模式不使用 `Input` widget，改用纯展示的文本渲染，以获得语法高亮能力。

### Raw 模式

- 显示 `response.body` 原始字节转换的字符串（不经任何格式化）
- 子格式选择器 (JSON/XML/Text/HTML) 在 Pretty 模式下不显示，仅在 Raw 模式下显示
- 点击子格式切换实际的 Input entity：
  - JSON → `response_input`（但内容应该是原始 body，非格式化版本）
  - XML → `response_xml_input`
  - Text → `response_text_input`
  - HTML → `response_html_input`
- 当前这些 entity 的存储值需要调整：Raw 模式下的 `response_input` 应保持原始 body，格式化的 JSON body 在 Pretty 模式渲染时动态格式化

**关键改动**：将 `response_input` 也存为原始 body，格式化逻辑从数据存储移到渲染层。这样 Raw 模式下所有 entity 都是原始文本，Pretty 模式渲染时才做 prettify。

响应到达时的数据填充：
```rust
// 所有 response_*_input 都存原始 body
this.response_input.update(cx, |state, cx| {
    state.set_value(&response.body, window, cx);
});
this.response_xml_input.update(...) // 原始 body
this.response_text_input.update(...) // 原始 body
this.response_html_input.update(...) // 原始 body
```

### Preview 模式

Content-Type 路由逻辑（新增函数 `get_preview_content_type`）：

```
image/png, image/jpeg, image/gif, image/webp, image/svg+xml, image/bmp, ...
  → 图片预览：gpui img() + object_fit(Contain)

text/html
  → 两个按钮："在浏览器中打开" / "查看源码"
  → "在浏览器中打开"：body 写入 tempfile → open::that() 用系统浏览器打开
  → "查看源码"：语法高亮 HTML 文本

application/json
  → 交互式 JSON 树形视图（新组件 json_tree_viewer）

application/pdf
  → "在外部程序中打开" 按钮（写入 tempfile → open::that()）

text/plain, text/css, text/javascript, application/xml, ...
  → 带语法高亮的文本预览（复用 Pretty 的渲染逻辑）

其他 / 未知类型
  → 尝试当作文本显示，或显示 "无法预览此内容类型"
```

### JSON 树形视图组件 (`src/ui/response/json_tree_viewer.rs`)

- 输入：JSON 字符串
- 用 `serde_json::from_str` 解析为 `serde_json::Value`
- 递归渲染树节点：
  - Object：`{` key: value ... `}`，每个 key 可折叠
  - Array：`[` item1, item2 ... `]`，可折叠
  - 叶节点：按类型着色（String=绿色, Number=蓝色, Boolean=橙色, Null=灰色）
- 折叠/展开：点击 `▶` / `▼` 箭头切换
- 每层缩进 16px
- 整个组件 `overflow_scroll()`

### 不再使用的代码

- `body_viewer.rs` 中的 `response_body_viewer()` 及相关函数 — 保持文件但标记 `#[allow(dead_code)]`，未来如需提取组件可参考
- 或者：直接删除 `response_body_viewer()` 和辅助函数（`view_mode_button`, `raw_format_selector`, `raw_format_button`, `raw_editor_content`, `raw_editor_panel`），因为它们从未被调用

推荐后者（删除死代码）。

## 依赖

- `open` crate — 用系统默认程序打开文件/URL（跨平台，已检查 Cargo.lock 未包含，需添加到 Cargo.toml）
- `serde_json` — 已有，JSON 解析
- `tempfile` — 或直接用 `std::env::temp_dir()` 手写临时文件路径，避免新依赖

## 风险

- **gpui img() 兼容性**：`img()` 的 API 在不同 gpui 版本间可能变化。本项目使用 zed-industries/zed 的 git 依赖，API 相对稳定。
- **大响应体**：JSON 树形视图对大 JSON（>1MB）可能导致性能问题。处理方式：超过阈值（如 500KB）时降级为普通文本显示。
- **临时文件泄露**：HTML/PDF 预览会创建临时文件。使用 `std::env::temp_dir()` 并以 `apipost-preview-*` 前缀命名，不做自动清理（用户重启系统会清掉）。
- **SVG 渲染**：gpui 的 `img()` 支持 SVG 格式，但复杂 SVG 可能渲染不正确。先按图片处理，有问题再迭代。

## 国际化和主题

- Pretty/Raw/Preview 按钮文本：已有 i18n key `ui.pretty`/`ui.raw`/`ui.preview`
- "Preview not available" / "Open in Browser" / "View Source" 等文本：新增 i18n key
- 语法高亮颜色：复用已有的 `Theme` 中的 `json_key`/`json_string`/`json_number`/`json_boolean`/`json_null`/`json_bracket` 颜色

## 测试

- 各 Content-Type 的预览路由决策
- `highlight_json()` 现有测试（如有）
- JSON 树形视图折叠/展开逻辑（单元测试）
- 不同字符集/编码的响应体处理
