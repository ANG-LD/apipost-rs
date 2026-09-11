//! 应用调色板 → gpui_component 组件库主题的桥接。
//!
//! 为什么要这一层：输入框、按钮、下拉框（Select）、设置弹窗这些控件是
//! gpui_component 渲染的，它们的颜色/圆角/焦点环全部读 `gpui_component::theme::Theme`
//! 这个全局量（见 crates/ui/src/input/input.rs、button/button.rs、
//! theme/schema.rs 的 `ThemeConfig::apply_config`），跟我们自己的 `Theme` 无关。
//! 不桥接的话，切到 gruvbox 这类主题时自绘面板是暖色、而输入框和下拉还是组件库
//! 默认的蓝灰配色，整屏像拼起来的。
//!
//! 覆盖方式（读源码确认过，不是猜的）：
//!
//! * `gpui_component::theme::Theme::global_mut(cx)` 给出 `&mut Theme`；
//! * 但**只改 `theme.colors.xxx` 是不够的** —— 按钮的 hover/active 取的是
//!   `theme.tokens.button_primary_hover` 这类缓存值，`tokens` 只在
//!   `ThemeConfig::apply_config` 里随 `colors` 一起刷新；
//! * 所以这里走 `Theme::global_mut(cx).apply_config(&Rc<ThemeConfig>)`（`apply_config`
//!   是 pub 的），一次把 colors + tokens + mode + radius + shadow 全部刷新；
//! * 输入框里的 **JSON 编辑器**（`InputState::code_editor("json")`）另有两条读者：
//!   正文/行号栏底色和行号颜色取自 `Theme::highlight_theme`，语法高亮取自
//!   `SyntaxHighlighter` + `cx.theme().highlight_theme`。`highlight_theme` 只在
//!   `ThemeConfig::highlight` 是 `Some` 时才会被重建，所以本层必须显式给出该字段
//!   （见 `highlight_style`），否则会一直停在组件库自带的浅色高亮主题上。
//! * 而且必须在 `gpui_component::init(cx)`（内部会 `Theme::change`）之后调用，
//!   否则会被 init 里的默认主题覆盖掉。

use super::tokens::{mix, RADIUS_LG, RADIUS_SM};
use super::Theme;
use gpui::{App, Rgba, SharedString};
use gpui_component::highlighter::HighlightThemeStyle;
use gpui_component::theme::{Theme as ComponentTheme, ThemeConfig, ThemeConfigColors, ThemeMode};
use std::rc::Rc;

/// 把 `Rgba` 转成组件库认的 `#rrggbbaa` 字面量（`gpui::Rgba::try_from` 支持该格式）。
fn hex(color: Rgba) -> SharedString {
    format!("#{:08x}", u32::from(color)).into()
}

// ==================== 对比度计算与兜底 ====================

/// sRGB 通道线性化（WCAG 2.x 的定义，不能直接用通道均值，否则深蓝/深红这类
/// 感知亮度与实际差异很大的颜色会被算错）。
fn linearize(channel: f32) -> f32 {
    let c = channel.clamp(0.0, 1.0);
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 相对亮度（0.0 最暗，1.0 最亮）。
fn relative_luminance(color: Rgba) -> f32 {
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

/// WCAG 对比度：1.0 表示两色完全相同，21.0 是黑白之差。
pub fn contrast_ratio(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// 编辑器里语法色的可读性下限。3.0 是 WCAG 对「大号文字/图形界面元素」的 AA 下限，
/// 也是这套调色板里 `json_null` 这类弱化色必须达到的水平。
pub const MIN_READABLE_CONTRAST: f32 = 3.0;

/// 兜底时多留的一点余量：颜色最终要量化成 8bit 十六进制再交给组件库，
/// 贴着阈值取色的话，1/255 的量化误差就能把实测值顶到 2.99。
const CONTRAST_SAFETY_MARGIN: f32 = 0.15;

/// 把颜色贴到 8bit 网格上。
///
/// 桥接层最终是通过 `#rrggbbaa` 十六进制把颜色交给组件库的（`hex()`），
/// 这里先量化一次，保证「断言里算对比度用的颜色」和「组件库真正拿到的颜色」
/// 是同一个值，不会出现断言通过、渲染出来却是另一个相近色相的情况。
/// 调色板里的颜色本来就是 8bit 精确值，量化对它们是恒等变换。
fn quantize(color: Rgba) -> Rgba {
    let q = |c: f32| (c.clamp(0.0, 1.0) * 255.0) as u32 as f32 / 255.0;
    Rgba {
        r: q(color.r),
        g: q(color.g),
        b: q(color.b),
        a: q(color.a),
    }
}

/// 对比度兜底：把 `color` 沿「向前景色方向」逐档混色，直到它压在 `background` 上够读。
///
/// 为什么需要：调色板里的 json_* 是给自绘视图（响应体高亮、JSON 树）挑的，
/// 那几处不叠加编辑器底色；直接搬给编辑器后，nord 的 `json_null`（1.36）、
/// light 的 `json_number`（2.89）这类颜色在编辑器底色上低于可读下限。
/// 之所以固定朝前景色方向混：前景色本身就是这套主题里「压在这个底色上一定读得清」
/// 的那个颜色，所以循环必然收敛，而且混出来的颜色仍属于同一套主题的色系。
pub fn readable_on(color: Rgba, background: Rgba, foreground: Rgba) -> Rgba {
    let target = MIN_READABLE_CONTRAST + CONTRAST_SAFETY_MARGIN;
    let mut mixed = quantize(color);
    let mut t = 0.0f32;
    while contrast_ratio(mixed, background) < target && t < 1.0 {
        t = (t + 0.02).min(1.0);
        mixed = quantize(mix(color, foreground, t));
    }
    mixed
}

// ==================== 编辑器底色家族 ====================

/// 编辑器（代码区）底色。
///
/// 与 `json_editor::code_editor_view` 传给 `Input::bg()` 的是同一个值：
/// 行号栏由组件库自己绘制，正文由应用传参，两边必须取同一份底色，
/// 否则行号栏和正文之间会出现一条色差缝。
pub fn editor_background_color(theme: &Theme) -> Rgba {
    theme.code_background
}

/// 行号栏（gutter）底色：在编辑器底色上掺一点前景色。
///
/// 为什么必须显式给：组件库的 `editor.gutter.background` 缺省回退到
/// `editor.background`，而我们不配置高亮主题时它会停在自带的浅色主题上（#ffffff），
/// 于是深色主题里出现一条白色行号栏。往前景色方向掺 7% 得到的颜色与编辑器底色
/// 同明暗体系（深色主题下比正文底亮一档、而不是发白），同时保留「行号栏」这一列
/// 的可辨识度：实测两者对比度 1.11~1.23，是有分隔感但不突兀的区间。
pub fn gutter_background_color(theme: &Theme) -> Rgba {
    mix(theme.code_background, theme.foreground, 0.07)
}

/// 当前行底色：在编辑器底色上掺一点主色。
///
/// 同样不能留空：留空时组件库用自带浅色主题的 #F5F5F5，深色主题下光标所在行
/// 会横贯一条浅色带。掺主色而不是掺前景色，是为了「当前行」看起来是强调而不是脏。
pub fn active_line_background_color(theme: &Theme) -> Rgba {
    mix(theme.code_background, theme.accent, 0.10)
}

/// 由调色板合成组件库的高亮主题（Zed 主题格式的 `highlight` 段）。
///
/// 为什么必须显式给全：`ThemeConfig::highlight` 是 `Option`，不给就完全不动
/// `Theme::highlight_theme`（见 crates/ui/src/theme/schema.rs 的 `apply_config`），
/// 于是它一直是 `HighlightTheme::default_light()`，两个症状都由此而来：
///
/// * 行号栏底色走 `editor.gutter.background` → 缺省回退 `editor.background`
///   → 浅色主题的 `#ffffff`（症状 1）；
/// * JSON 的键/字符串/数字用的是浅色主题的深蓝、深绿（`#0433ff` 之类），
///   压在深色 `code_background` 上几乎看不见（症状 2）。
///
/// `ThemeStyle` 的字段是私有的、没有构造函数，只能按组件库自己的主题格式
/// （也就是 Zed 主题的 JSON 结构）反序列化出 `HighlightThemeStyle`。
///
/// 只映射 JSON 语法树真正会产生的捕获名（见
/// crates/ui/src/highlighter/languages/json/highlights.scm）；没映射的捕获名
/// 会落到 `HighlightStyle::default()`，即继承应用设置的前景色，同样读得清 ——
/// 所以这里不做「猜一个语义相近的颜色」式的映射。
fn highlight_style(theme: &Theme) -> HighlightThemeStyle {
    let background = editor_background_color(theme);
    let gutter = gutter_background_color(theme);
    let foreground = theme.foreground;
    // 每个语法色都过一遍可读性兜底：调色板里的色值不动（自绘的响应体高亮、
    // JSON 树仍按原色渲染），只在编辑器这一侧按需提亮/压暗
    let syntax = |color: Rgba| serde_json::json!({ "color": hex(readable_on(color, background, foreground)) });

    let value = serde_json::json!({
        "editor.background": hex(background),
        "editor.foreground": hex(foreground),
        "editor.gutter.background": hex(gutter),
        // 行号是画在行号栏底色上的，兜底要拿行号栏当背景算，而不是编辑器正文底色
        "editor.line_number": hex(readable_on(theme.muted_foreground, gutter, foreground)),
        "editor.active_line_number": hex(foreground),
        "editor.active_line.background": hex(active_line_background_color(theme)),
        // 空白字符提示：跟着弱化文字色走，带透明度以免抢正文
        "editor.invisible": hex(theme.muted_foreground.opacity(0.4)),
        "syntax": {
            // 键
            "property": syntax(theme.json_key),
            // 字符串值 / 转义序列
            "string": syntax(theme.json_string),
            "string.escape": syntax(theme.json_string),
            // 数字
            "number": syntax(theme.json_number),
            // true / false
            "boolean": syntax(theme.json_boolean),
            // null 在 JSON 语法树里是 `constant.builtin`，
            // `SyntaxColors::style` 找不到精确名字时会回退到前缀 `constant`
            "constant": syntax(theme.json_null),
            // 括号、冒号、逗号
            "punctuation": syntax(theme.json_bracket),
            "punctuation.bracket": syntax(theme.json_bracket),
            "punctuation.delimiter": syntax(theme.json_bracket),
            "punctuation.special": syntax(theme.json_bracket),
            // 注释（JSONC 用得上）与关键字（配置里将来启用其它语言时用得上）
            "comment": syntax(theme.muted_foreground),
            "comment.doc": syntax(theme.muted_foreground),
            "keyword": syntax(theme.accent),
        },
    });

    // 结构是上面这个字面量，字段名/类型都由组件库定义，解析失败只可能是改错了这里
    serde_json::from_value(value).expect("highlight 主题结构固定，反序列化不会失败")
}

/// 由应用调色板合成一份组件库主题配置。
pub(crate) fn component_config(theme: &Theme) -> ThemeConfig {
    let mode = if Theme::is_light(&theme.name) {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };

    // ThemeConfigColors 里有私有字段（base.blue 等基础色），不能整体用结构体字面量 +
    // `..Default::default()` 构造，这里逐一赋值。
    let mut colors = ThemeConfigColors::default();

    // —— 表面与文字 ——
    colors.background = Some(hex(theme.background));
    colors.foreground = Some(hex(theme.foreground));
    colors.border = Some(hex(theme.border));
    // input 是「输入框边框色」，同时也是输入框底色的混色基准
    // （Theme::input_background() = input 混透明 30%），所以取主题边框色
    colors.input = Some(hex(theme.border));
    colors.muted = Some(hex(theme.muted_background));
    colors.muted_foreground = Some(hex(theme.muted_foreground));

    // —— 主色：主按钮、选中项、光标、焦点环、链接 ——
    colors.primary = Some(hex(theme.accent));
    colors.primary_foreground = Some(hex(theme.accent_foreground));
    colors.caret = Some(hex(theme.accent));
    colors.ring = Some(hex(theme.accent));
    // 选区：组件库内部会把 alpha clamp 到 0.3，不会糊住文字
    colors.selection = Some(hex(theme.accent));
    colors.link = Some(hex(theme.accent));
    colors.link_hover = Some(hex(theme.accent));
    colors.link_active = Some(hex(theme.accent));

    // —— 次级面 / hover 面 ——
    colors.secondary = Some(hex(theme.muted_background));
    colors.secondary_foreground = Some(hex(theme.foreground));
    // 组件库的 accent 用在 MenuItem / ListItem 的 hover 底色上，
    // 所以不能塞主题主色（那会变成一整片高饱和），用 muted_background
    colors.accent = Some(hex(theme.muted_background));
    colors.accent_foreground = Some(hex(theme.foreground));

    // —— 浮层：下拉面板、popover、列表 ——
    colors.popover = Some(hex(theme.background));
    colors.popover_foreground = Some(hex(theme.foreground));
    colors.list = Some(hex(theme.background));
    colors.list_even = Some(hex(theme.background));
    colors.list_head = Some(hex(theme.muted_background));
    colors.list_hover = Some(hex(theme.muted_background));
    colors.list_active = Some(hex(theme.accent));
    colors.list_active_border = Some(hex(theme.accent));

    // —— 语义色 ——
    colors.danger = Some(hex(theme.error));
    colors.success = Some(hex(theme.success));
    colors.warning = Some(hex(theme.warning));
    colors.info = Some(hex(theme.accent));

    // —— 滚动条：轨道跟底色、滑块跟边框，hover 再亮一档 ——
    colors.scrollbar = Some(hex(theme.background));
    colors.scrollbar_thumb = Some(hex(theme.border));
    colors.scrollbar_thumb_hover = Some(hex(theme.muted_foreground));

    // —— 窗口/标题栏边框与遮罩 ——
    colors.window_border = Some(hex(theme.border));
    colors.title_bar_border = Some(hex(theme.border));
    colors.overlay = Some(hex(theme.scrim()));

    ThemeConfig {
        // 名字带上应用主题名，组件库内部日志/调试时能一眼看出当前用的是哪套
        name: format!("apipost-{}", theme.name).into(),
        mode,
        // 圆角跟设计令牌走：常规控件 6px、弹窗 12px
        radius: Some(RADIUS_SM as usize),
        radius_lg: Some(RADIUS_LG as usize),
        shadow: Some(true),
        colors,
        // 语法高亮 + 编辑器底色家族（正文底、行号栏底、当前行、行号、JSON 语义色）。
        // `apply_config` 只有在这里是 `Some` 时才会重建 `Theme::highlight_theme`，
        // 留空就会退回组件库自带的浅色高亮主题。
        highlight: Some(highlight_style(theme)),
        ..Default::default()
    }
}

/// 把应用主题套到 gpui_component 上（颜色 + tokens + 模式 + 圆角 + 阴影一起刷新）。
///
/// 调用点只有两处：`main.rs` 启动时、`MainView::switch_theme` 切换主题时。
/// 两处都必须在 `gpui_component::init(cx)` 之后。
pub fn apply_component_theme(theme: &Theme, cx: &mut App) {
    let config = component_config(theme);

    // 兜底：万一调用顺序变成「先桥接、后 init」，先让组件库自己建好全局量，
    // 否则 global_mut 会 panic。
    if !cx.has_global::<ComponentTheme>() {
        ComponentTheme::change(config.mode, None, cx);
    }

    ComponentTheme::global_mut(cx).apply_config(&Rc::new(config));
    // apply_config 不重绘窗口；设置弹窗这类组件库控件需要显式刷新才会换色
    cx.refresh_windows();
}
