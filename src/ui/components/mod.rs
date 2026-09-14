//! UI组件模块

pub mod popup_panel;
pub use popup_panel::*;

use crate::ui::Theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{Icon, IconName, Size, Sizable};

/// HTTP方法颜色 (返回RGB值)
pub fn method_color(method: &str) -> u32 {
    match method.to_uppercase().as_str() {
        "GET" => 0x22c55e,      // 绿色
        "POST" => 0xf59e0b,    // 黄色
        "PUT" => 0x3b82f6,     // 蓝色
        "DELETE" => 0xef4444,  // 红色
        "PATCH" => 0x8b5cf6,   // 紫色
        "HEAD" => 0x6b7280,    // 灰色
        "OPTIONS" => 0x06b6d4, // 青色
        _ => 0x6b7280,
    }
}

/// 方法下拉（Select）的列表项。
///
/// gpui-component 的 Select 触发器内部有自己的文字样式，外层 `.text_color()`
/// 不会作用到"选中文字"上；只有通过列表项的 `display_title()`/`render()`
/// 返回自带颜色的元素，方法颜色才跟得上。
#[derive(Clone)]
pub struct MethodItem {
    pub name: SharedString,
    pub color: Rgba,
}

impl MethodItem {
    pub fn new(name: impl Into<SharedString>) -> Self {
        let name: SharedString = name.into();
        let color = rgb(method_color(&name));
        Self { name, color }
    }
}

impl gpui_component::searchable_list::SearchableListItem for MethodItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(
            div()
                .text_color(self.color)
                .child(self.name.clone())
                .into_any_element(),
        )
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().text_color(self.color).child(self.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

/// HTTP方法背景色 (返回RGB值)
pub fn method_bg_color(method: &str) -> u32 {
    match method.to_uppercase().as_str() {
        "GET" => 0x22c55e20,      // 绿色 20%透明度
        "POST" => 0xf59e0b20,     // 黄色 20%透明度
        "PUT" => 0x3b82f620,      // 蓝色 20%透明度
        "DELETE" => 0xef444420,  // 红色 20%透明度
        "PATCH" => 0x8b5cf620,   // 紫色 20%透明度
        "HEAD" => 0x6b728020,    // 灰色 20%透明度
        "OPTIONS" => 0x06b6d420, // 青色 20%透明度
        _ => 0x6b728020,
    }
}

/// 状态码颜色 (返回RGB值)
pub fn status_color(status: u16) -> u32 {
    if (200..300).contains(&status) {
        0x22c55e // 绿色 - 成功
    } else if (300..400).contains(&status) {
        0xf59e0b // 黄色 - 重定向
    } else if (400..500).contains(&status) {
        0xf97316 // 橙色 - 客户端错误
    } else if (500..600).contains(&status) {
        0xef4444 // 红色 - 服务器错误
    } else {
        0x6b7280 // 灰色 - 未知
    }
}

// ==================== 统一尺寸与样式令牌 ====================
// 尺寸常量与语义色的唯一来源是 `crate::ui::themes::tokens`（圆角、控件高度、
// hover/pressed/focus 配色都在那里定义）；这里只做重新导出，
// 让历史调用点 `crate::ui::components::CONTROL_H` 这类写法继续可用，不需要改动。
pub use crate::ui::themes::tokens::*;

/// 分段按钮组外框：把组内按钮包成一个整体（浅底 + 2px 内边距）
pub fn segment_group(theme: &Theme) -> gpui::Div {
    // 组容器用 hover 面（muted_background）而不是 code_background：
    // 前者才是「比页面浅一档的凹槽」这一语义，后者是代码块底色
    let bg = theme.hover_bg();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP_XS))
        .p(px(2.0))
        .rounded(px(RADIUS_SM))
        .bg(bg)
}

/// 分段按钮（单选按钮组单项）：
/// 请求体类型、JSON/XML/Text/HTML、响应的 格式化/原始/预览 等全部共用这一个样式。
pub fn segment_button(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    active: bool,
    theme: &Theme,
) -> Stateful<Div> {
    let accent = theme.accent;
    let accent_fg = theme.accent_foreground;
    let accent_hover = theme.accent_hover();
    let accent_pressed = theme.accent_pressed();
    let muted_fg = theme.muted_foreground;
    let fg = theme.foreground;
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    div()
        .id(id)
        .h(px(SEGMENT_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        // 轨道圆角 6px、内边距 2px，内项圆角取 6-2=4px 才贴合（原来是 5px，会露边）
        .rounded(px(RADIUS_XS))
        .text_size(px(12.0))
        .cursor_pointer()
        .when(active, |d| {
            d.bg(accent)
                .text_color(accent_fg)
                .font_weight(FontWeight(600.0))
                .shadow_sm()
                // 选中项也要有 hover/pressed 反馈，否则鼠标移上去像没响应
                .hover(move |s| s.bg(accent_hover))
                .active(move |s| s.bg(accent_pressed))
        })
        .when(!active, |d| {
            d.text_color(muted_fg)
                .hover(move |s| s.bg(hover_bg).text_color(fg))
                .active(move |s| s.bg(active_bg).text_color(fg))
        })
        .child(label)
}

/// 面板标签页（请求构造器的 参数/认证/请求头…、响应区的 响应体/响应头… 共用）：
/// 统一高度 + 选中项用底部 2px 主色下划线，而不是整块底色，视觉更干净。
pub fn pane_tab(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    active: bool,
    theme: &Theme,
) -> Stateful<Div> {
    let accent = theme.accent;
    let fg = theme.foreground;
    let muted_fg = theme.muted_foreground;
    let hover_bg = theme.muted_background;
    div()
        .id(id)
        .h(px(TAB_H))
        .px(px(GAP_L))
        .flex()
        .items_center()
        .text_size(px(12.0))
        .cursor_pointer()
        .border_b(px(2.0))
        .border_color(if active {
            accent
        } else {
            // 未选中也留 2px 透明下划线，保证切换时文字不上下跳动
            accent.alpha(0.0)
        })
        .text_color(if active { fg } else { muted_fg })
        .font_weight(if active {
            FontWeight(600.0)
        } else {
            FontWeight(400.0)
        })
        .hover(move |s| {
            if active {
                s
            } else {
                s.bg(hover_bg).text_color(fg)
            }
        })
        .child(label)
}

/// 主按钮（主色实心），用于「发送」等主要动作
pub fn primary_button(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    theme: &Theme,
) -> Stateful<Div> {
    let accent = theme.accent;
    let accent_fg = theme.accent_foreground;
    let accent_hover = theme.accent_hover();
    let accent_pressed = theme.accent_pressed();
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_L))
        .flex()
        .items_center()
        .justify_center()
        // 图标与文字之间一律 ICON_TEXT_GAP（4px）：以前主按钮是 8px、
        // 紧凑主按钮是 4px、各处调用点还有 2px，同一个"图标+文字"三种间距
        .gap(px(ICON_TEXT_GAP))
        .rounded(px(RADIUS_SM))
        .bg(accent)
        .text_color(accent_fg)
        .text_size(px(12.0))
        .font_weight(FontWeight(600.0))
        .cursor_pointer()
        .shadow_sm()
        // 以前是 opacity(0.9)：整体变半透明，叠在面板底色上像「褪色」而不是「亮起来」，
        // 还和 ghost_button 的 hover 观感不统一。改成主色与前景色混一档。
        .hover(move |s| s.bg(accent_hover))
        .active(move |s| s.bg(accent_pressed))
        .child(label)
}

/// 紧凑主色按钮：用在区块标题行里（如弹窗的「添加变量」），比 primary_button 更小
pub fn primary_button_sm(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    theme: &Theme,
) -> Stateful<Div> {
    let accent = theme.accent;
    let accent_fg = theme.accent_foreground;
    let accent_hover = theme.accent_hover();
    let accent_pressed = theme.accent_pressed();
    div()
        .id(id)
        .h(px(SEGMENT_H + 4.0))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(ICON_TEXT_GAP))
        .rounded(px(RADIUS_SM))
        .bg(accent)
        .text_color(accent_fg)
        .text_size(px(11.0))
        .font_weight(FontWeight(600.0))
        .cursor_pointer()
        .hover(move |s| s.bg(accent_hover))
        .active(move |s| s.bg(accent_pressed))
        .child(label)
}

/// 次级按钮（描边样式）：比原来的「灰底按钮」在深色主题下更清楚是按钮
pub fn ghost_button(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    theme: &Theme,
) -> Stateful<Div> {
    let border = theme.border;
    let fg = theme.foreground;
    let muted_fg = theme.muted_foreground;
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(ICON_TEXT_GAP))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(border)
        .text_color(muted_fg)
        .text_size(px(12.0))
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(fg))
        .active(move |s| s.bg(active_bg).text_color(fg))
        .child(label)
}

/// 开关（替代原来的 ON/OFF 文字块）：绿色/主色轨道 + 圆点
pub fn toggle_switch(
    id: impl Into<ElementId>,
    value: bool,
    theme: &Theme,
) -> Stateful<Div> {
    let on_color = theme.success;
    let off_color = theme.border;
    let knob = theme.accent_foreground;
    // 轨道的常态色本身就是语义色（success / border），所以 hover / 按下按同一套
    // 「在底色上叠一档前景色」的派生规则加深，颜色仍然全部来自主题
    let track = if value { on_color } else { off_color };
    let hover_track = theme.tint_hover(track);
    let active_track = theme.tint_active(track);
    div()
        .id(id)
        .w(px(34.0))
        .h(px(18.0))
        .flex()
        .items_center()
        .when(value, |d| d.justify_end())
        .when(!value, |d| d.justify_start())
        .px(px(2.0))
        .rounded_full()
        .bg(track)
        .cursor_pointer()
        // 开关本身是可点击控件：原来只有 cursor_pointer，点上去没有任何反馈
        .hover(move |s| s.bg(hover_track))
        .active(move |s| s.bg(active_track))
        .child(div().w(px(14.0)).h(px(14.0)).rounded_full().bg(knob).shadow_sm())
}

/// 行内「启用 / 禁用」开关（列表行左侧的 ✓ / ○ 方块）的前景色。
///
/// 两种状态各有一个语义色，只有这一份定义：
/// ✓ = 启用 = `success`，○ = 禁用 = `muted_foreground`。
/// 抽成函数是为了让 `theme_tests` 能拿**同一个来源**去断言两种状态压在
/// hover / 按下底色上的可读性 —— 测试与实现读同一处，才不会各说各话。
pub fn toggle_chip_foreground(enabled: bool, theme: &Theme) -> Rgba {
    if enabled {
        theme.success
    } else {
        theme.muted_foreground
    }
}

/// 行内「启用 / 禁用」开关在 hover / 按下时的前景色。
///
/// 两种状态统一换成 `foreground`（正文色）：反馈底色是 muted_background 系，
/// 实测 12 套主题里 `foreground` 压在这两个底色上最低 5.52（latte，测试里打印），
/// 而保留状态色会掉到 `muted_foreground × active_bg` 的 1.40（dracula）——
/// 那样"补反馈"反而把 ○ 看没了。状态本身由字形表达（✓ / ○）：
/// 在 light / sepia / latte 这几套主题里，success 与 muted_foreground 的对比
/// 本来就只有 1.0~1.3，区分状态从来靠的是字形。
pub fn toggle_chip_hover_foreground(theme: &Theme) -> Rgba {
    theme.foreground
}

/// 行内「启用 / 禁用」开关：24×24 的 ✓ / ○ 方块。
///
/// 参数 / 请求头 / 表单数据三个面板的行内开关共用这一份实现，于是两条规则
/// 只需要写一遍：
/// 1. **必须带唯一 id** —— gpui 的 `.hover()` / `.active()` 只在元素有
///    `global_id`（即 `.id()`）时才参与样式计算，而 id 只有在列表项里带上该项
///    下标/参数名时才唯一；共用 id 会变成「悬停一行、所有行一起高亮」；
/// 2. hover / 按下的底色一律取设计令牌（`hover_bg()` / `active_bg()`），不许写死颜色。
///
/// 方块尺寸 / 文字档位与改造前一致（24×24 + `text_sm`），只多了反馈底色与圆角。
pub fn enabled_toggle(id: impl Into<ElementId>, enabled: bool, theme: &Theme) -> Stateful<Div> {
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    let foreground = toggle_chip_foreground(enabled, theme);
    let feedback_foreground = toggle_chip_hover_foreground(theme);
    div()
        .id(id)
        .w(px(24.0))
        .h(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        // 反馈底色是 24×24 的小色块，圆角跟着「小元素」档走（RADIUS_XS），
        // 免得在全是圆角小块的界面里出现一个方角
        .rounded(px(RADIUS_XS))
        .cursor_pointer()
        .text_color(foreground)
        // hover / 按下：底色给反馈，同时把字换成正文色保证压得住（见上面的说明）
        .hover(move |s| s.bg(hover_bg).text_color(feedback_foreground))
        .active(move |s| s.bg(active_bg).text_color(feedback_foreground))
        .child(if enabled { "✓" } else { "○" })
}

// ==================== 表单数据行的 true / false 值切换 chip ====================
// 底色 = 状态色（success / error）按 alpha 叠在面板底色上。三档浓度只有这一份定义：
// `theme_tests` 拿同一组常量按 12 套主题断言"补了 hover/按下反馈之后，
// true / false 的字仍然够读"——测试与实现读同一处，才不会各改各的。

/// 常态浓度（与改造前一致）
pub const VALUE_CHIP_BASE_ALPHA: f32 = 0.14;
/// 悬停浓度：比常态浓一档（浓度越高越"亮"，足以看出反馈）
pub const VALUE_CHIP_HOVER_ALPHA: f32 = 0.22;
/// 按下浓度：比 hover 再浓一档
pub const VALUE_CHIP_PRESSED_ALPHA: f32 = 0.32;

/// 设置面板等处的区块标题
pub fn section_title(label: impl Into<SharedString>, theme: &Theme) -> gpui::Div {
    let color = theme.muted_foreground;
    div()
        .w_full()
        .pb(px(GAP_XS))
        .text_size(px(11.0))
        .font_weight(FontWeight(600.0))
        .text_color(color)
        .child(label.into())
}

/// 区块之间的分隔线
pub fn section_divider(theme: &Theme) -> gpui::Div {
    let color = theme.border;
    div().w_full().h(px(1.0)).my(px(2.0)).bg(color)
}


// ==================== 图标统一入口 ====================
// 背景：图标以前是 `Icon::new(x).xsmall()` / `.small()` 直接散在 12 个文件里，
// 尺寸档、语义色、要不要跟随容器 hover 全靠各自记忆 —— 于是同一个语义（比如"次级"）
// 在不同面板里出现过 muted_foreground / foreground / border 三种颜色。
// 这里把「尺寸（只有两档）+ 语义色（只有 6 档）」收敛成一次函数调用，
// 调用点只需要回答两个问题：这个图标属于哪一档尺寸、表达哪种语义。
//
// 两条与 gpui 渲染机制有关的硬约束（改代码前先读这两条）：
// 1. `Icon` 的尺寸不写就退化成"当前窗口文字样式的字号"，同一个图标在不同面板里
//    会跟着上下文变大小 —— 所以必须显式给尺寸，且只用下面两档。
// 2. `Icon` 的颜色如果没写，读的是"渲染时窗口的文字色"（会被祖先的 text_color 影响）；
//    一旦图标自己写了 text_color，祖先 hover/选中分支里的 text_color 就再也作用不到它。
//    所以"容器 hover 会换文字色"的位置必须用 `IconTone::Inherit`。

/// 图标尺寸档位：全应用只有两档，数值来自 `tokens.rs`。
///
/// 分档标准是**图标在界面里的角色**，不是随手挑的数字：
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconTier {
    /// 密集档 12px —— 凡是"跟着一行内容走"的图标：
    /// 列表/表格行里的图标、面板与弹窗里的小标题（侧栏标题、节标题、表单标签）、
    /// 徽章、按钮内部的图标、行内图标按钮（ICON_BTN 22 与 16–24px 微方框，
    /// 含弹窗标题栏的关闭按钮）、状态栏等细行里的图标。
    Dense,
    /// 标准档 14px —— 凡是"独立占一块地方"的图标：
    /// 弹窗标题图标、工具栏/标签栏/侧栏 tab 上的图标按钮（容器 ≥28px）、
    /// 图标徽章（36px 方框）。
    Regular,
}

impl IconTier {
    /// 档位 → 像素值。图标尺寸的唯一出口，别在调用点写数字。
    pub fn px(self) -> f32 {
        match self {
            IconTier::Dense => ICON_SIZE_SM,
            IconTier::Regular => ICON_SIZE_MD,
        }
    }
}

/// 图标语义色档：同一语义在全应用必须同色。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconTone {
    /// 继承容器文字色。
    ///
    /// **容器在 hover / 选中 / 按下时会换文字色的图标，必须用这一档**：
    /// `Icon` 渲染时读的是"当前窗口文字色"，只要图标自己写了 `text_color`，
    /// 容器的 hover 分支就再也作用不到它 —— 于是出现"文字亮了、图标还是灰的"。
    /// 另外实心主色按钮（primary_button 传 accent_foreground）、
    /// 危险按钮（danger_button 传 error）里的图标也走这一档：
    /// 它们的颜色由按钮本身决定，写死反而会和按钮参数脱钩。
    Inherit,
    /// 装饰性 / 次级：子标题前的图标、"更多…"、折叠箭头 —— 不抢视线
    Muted,
    /// 主要动作 / 与正文同级：正文里当"正文部件"用的图标
    Primary,
    /// 当前选中 / 强调：当前环境、选中项、品牌标识
    Accent,
    /// 危险动作：删除
    Danger,
    /// 成功 / 确认：校验通过、更新完成
    Success,
}

/// 语义色 → 调色板字段的唯一映射。
///
/// 拆成独立函数是为了可测：`icon_tone_color` 的返回值可以直接对调色板断言，
/// 而 `themed_icon` 生成的 `Icon` 内部颜色字段是私有的、测不到。
pub fn icon_tone_color(tone: IconTone, theme: &Theme) -> Option<Rgba> {
    match tone {
        IconTone::Inherit => None,
        IconTone::Muted => Some(theme.muted_foreground),
        IconTone::Primary => Some(theme.foreground),
        IconTone::Accent => Some(theme.accent),
        IconTone::Danger => Some(theme.error),
        IconTone::Success => Some(theme.success),
    }
}

/// 组件库 `Button` 内部图标的尺寸换算比例。
///
/// 为什么需要这个比例（读 `gpui-component/crates/ui/src/button/button.rs` 得到）：
/// `RenderOnce for Button` 里 `icon_size = size * 0.75`，紧接着
/// `.with_size(icon_size)` **覆盖**调用方塞进 `ButtonIcon` 的尺寸 ——
/// 也就是说按钮里图标多大，只由 `Button` 自身的 size 决定，`.icon()` 传进去的尺寸无效。
/// 于是"让按钮里的图标也落在 12/14px 两档"只能反过来解：Button size = tier.px() / 0.75。
pub const BUTTON_ICON_RATIO: f32 = 0.75;

/// `IconTier` → 组件库 `Button` 的 `Size`，使按钮内的图标恰好等于该档像素值。
///
/// 不用 `.xsmall()/.small()`：那两档给的是 `size_3()/size_3p5()`（rem 档位），
/// 会随主题字号漂移，与本项目"图标尺寸是绝对值、只有两档"的规则冲突。
pub fn button_size_for_icon(tier: IconTier) -> Size {
    Size::Size(px(tier.px() / BUTTON_ICON_RATIO))
}

/// 生成一个符合本应用图标规则的图标：尺寸取两档之一，颜色取语义色表。
///
/// 全应用只有这一个构造出口 —— 不再有"忘记写尺寸"的图标
/// （不写尺寸时 `Icon` 会退化成当前窗口文字的字号，等于把尺寸交给上下文，
/// 同一个图标在不同面板里就会不一样大）。
pub fn themed_icon(name: IconName, tier: IconTier, tone: IconTone, theme: &Theme) -> Icon {
    apply_icon_style(Icon::new(name), tier, tone, theme)
}

/// 自绘图标（不在组件库图标集里）的构造出口，与 `themed_icon` 共用同一套规则。
///
/// 为什么要单独一个入口：`themed_icon` 收的是组件库的 `IconName`，而自绘图标
/// 没有对应的枚举项，只有一个资源路径。但"尺寸档 + 语义色"必须仍然只有一套规则 ——
/// 另起一套就又会漂移成"同一语义两个颜色"。所以这里只换"字形从哪来"，
/// 尺寸与颜色仍旧走 `apply_icon_style`。
///
/// `path` 是资源源里的路径，必须真的能被 `AssetSource` 解析到：
/// 组件库图标走组件库那份资源源，自绘图标走 `crate::assets` 的桥接
/// （清单见 `crate::assets`，例如 `HISTORY_ICON_PATH`）。
pub fn themed_svg_icon(path: &str, tier: IconTier, tone: IconTone, theme: &Theme) -> Icon {
    apply_icon_style(Icon::empty().path(path), tier, tone, theme)
}

/// `themed_icon` / `themed_svg_icon` 共用的尺寸与语义色映射 —— 全应用图标的唯一规则点
fn apply_icon_style(icon: Icon, tier: IconTier, tone: IconTone, theme: &Theme) -> Icon {
    let icon = icon.with_size(px(tier.px()));
    match icon_tone_color(tone, theme) {
        Some(color) => icon.text_color(color),
        // Inherit：刻意不写 text_color，让 Icon 用窗口文字色渲染
        // （`RenderOnce for Icon` 里读 `window.text_style().color`），
        // 这样容器 hover / 选中换文字色时图标跟着变
        None => icon,
    }
}

/// 带图标的次级按钮（如「＋ 添加参数」「＋ 添加请求头」）：描边 + 图标 + 文字
pub fn icon_text_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    theme: &Theme,
    text_color: Rgba,
) -> Stateful<Div> {
    let border = theme.border;
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    let fg = theme.foreground;
    div()
        .id(id)
        // 原来是 CONTROL_H - 4.0（28px），正好等于紧凑控件高度令牌，改成直接引用令牌
        .h(px(CONTROL_H_SM))
        .px(px(GAP_M))
        .flex()
        .items_center()
        // 父容器是列方向 flex，按钮会被拉伸到整行宽；内容居中才不会出现
        // "按钮很宽、文字却贴左边" 的观感
        .justify_center()
        .gap(px(ICON_TEXT_GAP))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(border)
        .text_size(px(12.0))
        .text_color(text_color)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(fg))
        .active(move |s| s.bg(active_bg).text_color(fg))
        // 图标不写颜色（Inherit）：按钮 hover/按下会换文字色，图标必须跟着换
        .child(themed_icon(icon, IconTier::Dense, IconTone::Inherit, theme))
        .child(label.into())
}

/// 表单行：固定宽度标签 + 控件，行高统一，多行天然左对齐
pub fn form_row(
    label: impl Into<SharedString>,
    theme: &Theme,
    control: AnyElement,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP_M))
        .h(px(FORM_ROW_H))
        .child(
            div()
                .w(px(FORM_LABEL_W))
                .flex_shrink_0()
                .text_sm()
                .text_color(theme.foreground)
                .child(label.into()),
        )
        .child(control)
        .into_any_element()
}

/// 行内图标按钮（列表行的「更多…」「删除」等次要操作），边长走 ICON_BTN 令牌
pub fn icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    theme: &Theme,
    color: Rgba,
    hover_color: Rgba,
) -> Stateful<Div> {
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    div()
        .id(id)
        .w(px(ICON_BTN))
        .h(px(ICON_BTN))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .text_color(color)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(hover_color))
        .active(move |s| s.bg(active_bg).text_color(hover_color))
        // 颜色由调用方传进来的 color/hover_color 决定，所以图标走 Inherit
        .child(themed_icon(icon, IconTier::Dense, IconTone::Inherit, theme))
}

/// 危险操作按钮（删除环境等）：描边 + 错误色文字，悬停时底色泛红、边框转错误色
pub fn danger_button(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    theme: &Theme,
) -> Stateful<Div> {
    let border = theme.border;
    let err = theme.error;
    // 悬停底色仍是错误色淡化；但文字色不能再用 accent_foreground ——
    // 浅色主题里它是白色，叠在淡红底上几乎看不见（原来就是这个 bug）。
    // 文字保持错误色，靠底色和边框变化表达悬停。
    let hover_bg = err.alpha(0.16);
    let active_bg = err.alpha(0.26);
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(ICON_TEXT_GAP))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(border)
        .text_size(px(12.0))
        .text_color(err)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).border_color(err))
        .active(move |s| s.bg(active_bg).border_color(err))
        .child(label)
}
