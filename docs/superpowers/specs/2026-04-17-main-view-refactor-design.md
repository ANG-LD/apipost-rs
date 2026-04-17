# MainView 重构设计方案

## 1. 目标

将 `main_view.rs`（2732行）拆分为多个独立组件，实现：
- 左右布局：左侧边栏 + 右侧主工作区
- 右边上下布局：请求构造器(上) + 响应查看器(下)
- Splitter可拖拽调整上下区域大小
- Params区域内部滚动查看

## 2. 布局结构

```
+-------------------------------------------------------------------+
|  状态栏                                                              |
+----------+--------------------------------------------------------+
|          |  请求标签栏 (标签1 标签2 标签3 ...)                      |
|  侧边栏   |  +------------------------------------------------+  |
|          |  | URL输入行 [GET ▼] [________URL________] [Send]   |  |
| Collections |  +------------------------------------------------+  |
| History |  | [Params][Auth][Headers][Body][Pre-req][Tests]     |  |
| Env     |  | +----------------------------------------------+ |  |
|          |  | | Params面板 (内部滚动)                        | |  |
|          |  | +----------------------------------------------+ |  |
|          |  +------------------------------------------------+  |
|          |  =========== Splitter (可拖拽) ====================  |
|          |  +------------------------------------------------+  |
|          |  | 响应查看器                                       |  |
|          |  | [Body][Cookies][Headers][Test Results]          |  |
|          |  | +----------------------------------------------+ |  |
|          |  | | 响应内容 (带滚动)                              | |  |
|          |  | +----------------------------------------------+ |  |
|          |  +------------------------------------------------+  |
+----------+--------------------------------------------------------+
```

## 3. 组件拆分

### 3.1 文件结构

| 文件 | 职责 | 行数估计 |
|------|------|----------|
| main_view.rs | 组装布局、状态管理、事件路由 | ~300 |
| sidebar.rs | 侧边栏、历史/集合/环境切换 | ~400 |
| request_builder.rs | 请求构造器整体（含URL、标签页、Params） | ~1200 |
| response_viewer.rs | 响应查看器（标签页、内容显示） | ~600 |
| splitter.rs | 拖拽分割条组件 | ~100 |

### 3.2 模块定义 (ui/mod.rs)

```rust
mod sidebar;
mod request_builder;
mod response_viewer;
mod splitter;

pub use sidebar::*;
pub use request_builder::*;
pub use response_viewer::*;
pub use splitter::*;
```

## 4. 组件详细设计

### 4.1 MainView (main_view.rs)

**职责：**
- 持有所有应用状态
- 组装整体布局
- 事件路由到子组件
- Splitter拖拽逻辑

**状态：**
```rust
pub struct MainView {
    app_state: Arc<AppState>,
    // 已有状态保持不变...
    splitter_dragging: bool,
    splitter_start_y: f32,
    request_height_ratio: f32,  // 0.0-1.0
}
```

**布局：**
```rust
div()
    .size_full()
    .flex()
    .flex_col()
    .children([
        status_bar(),
        div()
            .flex_1()
            .flex()
            .flex_row()
            .children([
                Sidebar::new(),
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .children([
                        request_tabs(),
                        RequestBuilder::new(),
                        Splitter::new(),
                        ResponseViewer::new(),
                    ]),
            ]),
    ])
```

### 4.2 Sidebar (sidebar.rs)

**职责：**
- 显示历史/集合/环境标签页
- 渲染历史记录列表
- 侧边栏折叠/展开

**Props：**
- `history: Vec<HistoryEntry>`
- `active_tab: SidebarTab`
- `collapsed: bool`
- `on_tab_change: Callback<SidebarTab>`
- `on_toggle: Callback<()>`

### 4.3 RequestBuilder (request_builder.rs)

**职责：**
- URL输入和方法选择
- Builder标签页切换（Params/Headers/Body等）
- Params面板（带内部滚动）
- 其他标签页内容

**Props：**
- 所有与请求相关的状态
- 回调事件到MainView

**Params面板内部滚动实现：**
```rust
div()
    .flex_1()
    .overflow_y_auto()  // 内部滚动
    .children([
        // 表头
        div().flex()...,
        // 参数行列表
        div()
            .flex_col()
            .children(params.iter().map(...)),
    ])
```

### 4.4 ResponseViewer (response_viewer.rs)

**职责：**
- 响应标签页（Body/Cookies/Headers/Test Results）
- 响应内容显示（Pretty/Raw/Preview）
- 状态、时间、大小信息

**Props：**
- `response: Option<HttpResponse>`
- `response_tab: ResponseTab`
- `body_view_mode: BodyViewMode`
- 回调事件

### 4.5 Splitter (splitter.rs)

**职责：**
- 可拖拽分割条
- 视觉反馈（hover颜色）

**Props：**
- `dragging: bool`
- `on_drag_start: Callback<f32>`
- `on_drag_update: Callback<f32>`
- `on_drag_end: Callback<()>`

## 5. Splitter拖拽逻辑

保持现有逻辑不变：

```rust
pub fn start_splitter_drag(&mut self, start_y: f32) {
    self.splitter_dragging = true;
    self.splitter_start_y = start_y;
}

pub fn update_splitter_drag(&mut self, current_y: f32) {
    if self.splitter_dragging {
        let delta_y = current_y - self.splitter_start_y;
        let ratio_delta = delta_y / 1000.0;
        self.request_height_ratio = (self.request_height_ratio + ratio_delta).max(0.2).min(0.8);
        self.splitter_start_y = current_y;
    }
}
```

## 6. 实现顺序

1. 创建 `splitter.rs` - 提取分割条组件
2. 创建 `sidebar.rs` - 提取侧边栏组件
3. 创建 `request_builder.rs` - 提取请求构造器（包含Params内部滚动）
4. 创建 `response_viewer.rs` - 提取响应查看器
5. 精简 `main_view.rs` - 保留布局组装和状态管理
6. 更新 `ui/mod.rs` - 导出新模块
7. 测试验证功能完整性

## 7. 注意事项

- 保持现有功能不变
- 事件回调机制与gpui一致
- Params面板使用 `overflow_y_auto()` 实现内部滚动
- Splitter拖拽比例限制在0.2-0.8之间
