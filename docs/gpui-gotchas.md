# gpui / gpui-component 踩坑记录

本文件记录在 apipost-rs 上实际踩到、并且**已经用测量数据验证过**的 gpui 坑。

- gpui: rev `1d217ee39d381ac101b7cf49d3d22451ac1093fe`
- gpui-component: `1505b14`

每条按「症状 → 根因 → 正确写法 → 代码位置 → 怎么验证」组织。

> ⚠️ 这些代码里有很多看起来"多余"的显式高度、`min_h(px(0.0))`、`flex_none()`、尾部留白，
> 它们不是冗余样式，删掉就会复现 bug。改之前请先读对应条目。

---

## 1. 侧边栏的高度链是 auto：滚动区必须显式给高度

**症状**：列表底部被窗口裁掉、滚轮完全没反应、也没有滚动条。

**根因**：在 `侧边栏列 → 侧边栏内容容器 → 各面板` 这条链上，`flex_1()` / `height: 100%`
**都解析不出确定高度**（渲染根到侧边栏这一段是 auto）。而且即使给了显式高度也会失效——
flex 子项的**自动最小高度 = 内容高度**，会把显式高度顶开。

实测（窗口 800px）：

| 探针读数 | 修复前 | 修复后 |
|---|---|---|
| 收藏夹滚动容器自身高 | **977px**（= 内容撑开，无界） | **710px**（有界） |
| 列表内容高 | 946px | 946px |

容器(977) > 窗口(800) 时，既被裁切、又因为"容器高 ≥ 内容高"导致可滚动范围是 0。

**正确写法**（三个条件缺一不可）：

```rust
// 1) 显式高度：按窗口高度算出可用高度，不要写死
let sidebar_content_h = (window.bounds().size.height.as_f32() - 121.0).max(160.0);

// 2) 容器：显式高度 + min_h(0) 解除 flex 自动最小高度 + flex_shrink_0
div().flex_col()
    .h(px(sidebar_content_h))
    .min_h(px(0.0))
    .flex_shrink_0()
    .overflow_hidden()

// 3) 滚动容器：同样显式高度（这里 flex_1/100% 都取不到父高度）
div().id("sidebar-collections-scroll")
    .h(px(sidebar_content_h)).min_h(px(0.0)).flex_shrink_0()
    .flex_col().overflow_y_scroll()
    .child(/* 内容层：w_full + flex_none，保持自然高度 */)
```

**代码位置**
- `src/ui/main_view.rs:3996` —— `sidebar_content_h` 计算
- `src/ui/main_view.rs:4232` —— 侧边栏内容容器
- `src/ui/main_view.rs:4419` 附近 —— 收藏夹滚动容器
- `src/ui/sidebar/environment_panel.rs:49 / 383 / 391` —— `ENV_TITLE_H`、根元素、环境滚动容器

**怎么验证**：探针挂在侧边栏内容容器上（见第 5 条），量它第一个子元素（= 分支根）的高度，
期望 ≈ `窗口高 - 121`；若远大于窗口高就是没约束住。

---

## 2. gpui-component 的 `Scrollable` 会把滚动范围压成 0

**症状**：给容器加"滚动条"（`overflow_y_scrollbar()`）之后，反而完全不能滚。

**根因**：`gpui_component::scroll::Scrollable` 的 `RenderOnce` 实现会把传进来的元素包成：

```
div().size_full()
  .child(div().id("scroll-area").flex().size_full().track_scroll().flex_col()
           .overflow_y_scroll()
           .child(element.size_auto().flex_1()))   // ← 问题在这
```

它给内容注入了 `.size_auto().flex_1()`，内容被压到容器高度，
于是 gpui 认为"内容高 == 容器高"，滚动范围为 0。

**正确写法**：侧边栏这类列表用**原生** `overflow_y_scroll()`，并让内容层保持自然高度：

```rust
div().id("history-scroll").size_full().flex_col().overflow_y_scroll()
    .child(div().id("history-list").w_full().flex_none().flex_col().gap_1().p_2().children(items))
```

注意 `flex_none()`（或 `flex_shrink_0()`）是关键：flex 子项默认 `flex_shrink: 1`，
不加就会被压缩到刚好装下，同样让滚动范围归零。

> 另外注意区分两个同名项：`gpui_component::scroll::Scrollable`（结构体）
> 与 `ScrollableElement`（trait，`overflow_y_scrollbar` 的提供者）。
> 引错会报 `no method named overflow_y_scrollbar found for struct Stateful<E>`。

**代码位置**：`src/ui/main_view.rs:4251`（历史）、`4419` 附近（收藏夹）

---

## 3. 滚动范围按「容器高度」算，不是按「看得见的区域」

**症状**：能滚，但**最后一条数据永远滚不出来**，停在底部被截断遮住。

**根因**：gpui 用容器自身高度算滚动范围。如果容器比实际可见区域高（多出来的部分被父层
`overflow_hidden` 裁掉），滚动范围就会虚高——滚到底时最后一条正好停在被裁掉的那一条带子里。
实测错位约 **31px**（容器声明 679px，实测 710px）。

**处理**：
- 兜底（当前采用）：内容层加尾部留白，让最后一条停在裁切带之上
  - 收藏夹 `src/ui/main_view.rs:4450` —— `.p_2().pb(px(40.0))`
  - 历史 `src/ui/main_view.rs:4263` —— `.p_2().pb(px(24.0))`
- 更彻底：把容器高度改成**等于实际可见高度**（而不是 `窗口高 - 121` 推算），
  这样就不需要靠留白兜底。需要时用第 6 条的探针把侧边栏底部固定占用的真实值量出来。

留白量只要大于那条错位带的高度就够；不够加几个像素即可。

---

## 4. `Select` 的「选中文字」颜色不跟随外层 `.text_color()`

**症状**：方法下拉明明写了 `.text_color(rgb(method_color(&method)))`，选中文字却还是主题前景色。

实测：代码设定 `method_color("GET") = 0x22c55e`（绿），截图里方法框文字像素是
`(212,212,212)`（前景灰）——颜色确实没生效。

**根因**：gpui-component 的 `Select` 触发器内部自带文字样式，外层元素的 `text_color`
作用不到"选中文字"上（可以理解为触发器的样式优先级更高）。

**正确写法**：颜色要挂在**列表项**上，实现 `SearchableListItem`：

```rust
impl gpui_component::searchable_list::SearchableListItem for MethodItem {
    type Value = SharedString;
    fn title(&self) -> SharedString { self.name.clone() }          // 兜底标题
    fn display_title(&self) -> Option<AnyElement> {                // 触发器里显示的文字
        Some(div().text_color(self.color).child(self.name.clone()).into_any_element())
    }
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {  // 下拉列表里的行
        div().text_color(self.color).child(self.name.clone())
    }
    fn value(&self) -> &Self::Value { &self.name }
}
```

库的触发器确实会调用它：`select.rs:432`（优先 `item.display_title()`）和 `select.rs:522`
（`.child(self.display_title(window, cx))`）。

**顺带一条**：**没设颜色的 `Select`**，选中文字会落到"占位符"的弱化色（`muted_foreground`），
看起来就是"不跟主题设定的颜色"。给 `Select` 补一句 `.text_color(theme.foreground)` 即可。

**代码位置**
- `src/ui/components/mod.rs:31 / 44` —— `MethodItem` 与其 trait 实现
- `src/ui/main_view.rs:283 / 806` —— `method_select` 类型与构造
- `src/ui/request/url_bar.rs`（方法下拉，颜色由列表项自带）
- `src/ui/request/body_panel.rs:406` —— 参数类型下拉补的 `.text_color(theme.foreground)`

---

## 5. `on_children_prepainted` 只存在于 `Div` 上

想在渲染阶段读子元素布局（打探针）时：

```rust
div()
    .on_children_prepainted(|bounds, window, _cx| { /* bounds: Vec<Bounds<Pixels>> */ })
    .id("some-id")          // ← 必须放在 .id() 之前
```

- 该 hook 只在 `Div` 上，`Stateful<Div>` 没有 → 写在 `.id(...)` 之后会报
  `no method named on_children_prepainted found for struct gpui::Stateful<E>`。
- `bounds` 是子元素的布局矩形，顺序与 `.child()/.children()` 一致；
  量"某个容器自身高度"就把它挂在父级、取 `bounds.first()`。

---

## 6. 这个项目的无头验证手法（很重要）

改 UI 之前先看这一节，能省掉大量来回。

**能做什么**
- **键盘事件有效**：Ctrl+T（切主题）、Ctrl+H（历史）等都能驱动应用。
- **布局探针**：用第 5 条的 hook 打日志，从
  `$XDG_DATA_HOME/apipost-rs/debug.log` 读数（建议每 30 帧打一次，避免刷屏）。
- **截图取像素**：`xwd -id $WID -silent > x.xwd && ffmpeg -y -i x.xwd x.png`，
  再用 numpy 统计颜色块位置/数量。常用颜色：
  背景 `0x0c0c1d`、次级底色 `0x18182a`、强调色 `0x6366f1`、
  前景文字 `(212,212,212)`、弱化文字 `(128,128,131)`、GET 绿 `0x22c55e`。

**不能做什么**
- **指针/滚轮事件进不到应用**：点击、悬停、滚轮都不会被处理。
  所以"点了没反应/没变化"**不能**当作 bug 的证据（这一点曾导致一次误判）。
  滚动只能靠"可滚动范围是否非零"这类布局数值间接验证。

**两个已经踩过的坑**
1. **探针区域要先核准**：曾把 URL 输入框的文字当成方法下拉的文字，误判成"颜色没生效"。
   定位前先用像素统计确认坐标范围里到底是哪个控件。
2. **临时改动要记得还原**：为了截图方便临时改过"默认页签""对话框强制 visible"等，
   收尾时用 `grep -rn TEMP src/` 自查，并重新 `cargo build` + `cargo test`。

**测试环境约定**（避免动到用户真实数据）

```bash
DISPLAY=:1 XDG_DATA_HOME=/tmp/dshui/appdata HOME=/tmp/dshui/home ./target/debug/apipost-rs
```

- 窗口：`xdotool search --pid $(cat /tmp/dshui/pid) | tail -1`
- 日志：`/tmp/dshui/appdata/apipost-rs/debug.log`
- 数据库：`/tmp/dshui/appdata/apipost-rs/database.db`（首次启动后可用 sqlite3 灌数据）
- 结束：`kill $(cat /tmp/dshui/pid)`，**不要** `pkill -f`（会匹配到自身），最后 `rm -rf /tmp/dshui`
- **不要**动 `~/.local/share/apipost-rs/database.db` 和 `~/.config/apipost-rs/config.toml`（用户真实数据）

---

### 补充（实测）：`max_h` 不足以让滚动区可滚动

设置浮层的内容区原来写成 `.max_h(px(max_height)).overflow_y_scroll()`，
看起来"高度有上限了"，但实测探针显示**容器高 511px == 内容高 511px，滚动范围 0**
（`max_h` 只约束外框，容器自身高度仍等于内容高度，而 gpui 按容器高度算滚动范围）。
改成显式定高 `.h(px(max_height - 38.0))` 后：**容器 398px / 内容 511px → 有 113px 范围**。

结论：**凡是滚动区，一律给显式 `h(px(..))`**，`max_h`/`flex_1`/`h_full` 都不算确定高度。
高度可以像侧边栏那样按窗口算：`(window.bounds().size.height - 常量)`。


## 一句话总结

这套组合里，「看不见 / 量不到」经常是**测量方式的问题**，而不是结论。
先量（探针 + 像素），再改；改完再量一次。
