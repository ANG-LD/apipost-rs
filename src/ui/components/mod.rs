//! UI组件模块

pub mod popup_panel;
pub use popup_panel::*;

use crate::ui::Theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{Icon, IconName, Sizable};

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
// 各面板以前各写一套按钮/标签样式（圆角、字号、内边距、高度都不一样），
// 这里集中定义，新增控件一律从这里取，保证界面观感一致。

/// 普通控件高度（输入框、主/次按钮）
pub const CONTROL_H: f32 = 32.0;
/// 分段按钮（按钮组内单项）高度
pub const SEGMENT_H: f32 = 22.0;
/// 面板标签页高度
pub const TAB_H: f32 = 36.0;
/// 小间距 / 中间距 / 大间距 / 面板内边距
pub const GAP_XS: f32 = 4.0;
pub const GAP_S: f32 = 8.0;
pub const GAP_M: f32 = 12.0;
pub const GAP_L: f32 = 16.0;
pub const PANEL_PAD: f32 = 12.0;

/// 分段按钮组外框：把组内按钮包成一个整体（浅底 + 2px 内边距）
pub fn segment_group(theme: &Theme) -> gpui::Div {
    let bg = theme.code_background;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP_XS))
        .p(px(2.0))
        .rounded_md()
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
    let muted_fg = theme.muted_foreground;
    let fg = theme.foreground;
    let hover_bg = theme.muted_background;
    div()
        .id(id)
        .h(px(SEGMENT_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .text_size(px(12.0))
        .cursor_pointer()
        .when(active, |d| {
            d.bg(accent)
                .text_color(accent_fg)
                .font_weight(FontWeight(600.0))
                .shadow_sm()
        })
        .when(!active, |d| {
            d.text_color(muted_fg)
                .hover(move |s| s.bg(hover_bg).text_color(fg))
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
            rgba(0x00000000)
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
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_L))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(GAP_XS))
        .rounded_md()
        .bg(accent)
        .text_color(accent_fg)
        .text_size(px(12.0))
        .font_weight(FontWeight(600.0))
        .cursor_pointer()
        .shadow_sm()
        .hover(|s| s.opacity(0.9))
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
    div()
        .id(id)
        .h(px(SEGMENT_H + 4.0))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(2.0))
        .rounded_md()
        .bg(accent)
        .text_color(accent_fg)
        .text_size(px(11.0))
        .font_weight(FontWeight(600.0))
        .cursor_pointer()
        .hover(|s| s.opacity(0.9))
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
    let hover_bg = theme.muted_background;
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(GAP_XS))
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_color(muted_fg)
        .text_size(px(12.0))
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(fg))
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
        .bg(if value { on_color } else { off_color })
        .cursor_pointer()
        .child(div().w(px(14.0)).h(px(14.0)).rounded_full().bg(knob).shadow_sm())
}

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


/// 带图标的次级按钮（如「＋ 添加参数」「＋ 添加请求头」）：描边 + 图标 + 文字
pub fn icon_text_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    theme: &Theme,
    text_color: Rgba,
) -> Stateful<Div> {
    let border = theme.border;
    let hover_bg = theme.muted_background;
    let fg = theme.foreground;
    div()
        .id(id)
        .h(px(CONTROL_H - 4.0))
        .px(px(GAP_M))
        .flex()
        .items_center()
        // 父容器是列方向 flex，按钮会被拉伸到整行宽；内容居中才不会出现
        // "按钮很宽、文字却贴左边" 的观感
        .justify_center()
        .gap(px(GAP_XS))
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_size(px(12.0))
        .text_color(text_color)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(fg))
        .child(Icon::new(icon).xsmall())
        .child(label.into())
}

/// 表单标签列宽 / 行高（设置、认证等表单类面板统一）
pub const FORM_LABEL_W: f32 = 160.0;
pub const FORM_ROW_H: f32 = CONTROL_H + 6.0;

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

/// 22x22 行内图标按钮（列表行的「更多…」「删除」等次要操作）
pub fn icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    theme: &Theme,
    color: Rgba,
    hover_color: Rgba,
) -> Stateful<Div> {
    let hover_bg = theme.muted_background;
    div()
        .id(id)
        .w(px(22.0))
        .h(px(22.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(color)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(hover_color))
        .child(Icon::new(icon).xsmall())
}

/// 危险操作按钮（删除环境等）：描边 + 错误色文字，悬停填充错误色
pub fn danger_button(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    theme: &Theme,
) -> Stateful<Div> {
    let border = theme.border;
    let err = theme.error;
    let mut hover_bg = theme.error;
    hover_bg.a = 0.16;
    let fg = theme.accent_foreground;
    div()
        .id(id)
        .h(px(CONTROL_H))
        .px(px(GAP_M))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(GAP_XS))
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_size(px(12.0))
        .text_color(err)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(fg))
        .child(label)
}
