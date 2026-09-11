//! 设计令牌（Design Tokens）
//!
//! 背景：圆角、控件高度、状态配色以前散落在二十多个面板文件里 —— 同一个「输入框」
//! 在不同面板里底色可能是 `code_background` 也可能是 `input_background`，hover 底色
//! 有的用 `code_background` 有的用 `muted_background`，弹窗圆角有 8px 也有 12px。
//! 这里把这类东西收敛成两类：
//!
//! 1. **与主题无关的尺寸常量**：圆角档位、控件高度、间距。写在下面，数值只有一份。
//! 2. **与主题有关的语义色**：一律由 `Theme` 现有字段派生（混色 / 透明度），
//!    所以 12 套主题（dark/light/sepia/ocean/sunset/forest/monokai/nord/dracula/
//!    tokyonight/gruvbox/latte）自动协调，不需要——也不允许——写死颜色。
//!
//! 新增控件时优先从这里取值；确需新档位时先在此处补常量，不要在面板里就地写数字。

use super::Theme;
use gpui::Rgba;

// ==================== 圆角档位 ====================
// 只有 4 档。数值刻意对齐 gpui 的 rounded_xs/sm/lg 与 rounded_xl（4/6/8/12），
// 以及 gpui_component 的 theme.radius(=6) / radius_lg，这样自绘控件和组件库控件
// 圆角天然一致，不会出现「输入框 6px、按钮 5px」这种肉眼可见的错位。

/// 标签、徽章、分段控件内部项（小元素用大圆角会显得胖）
pub const RADIUS_XS: f32 = 4.0;
/// 输入框、按钮、行内图标按钮 —— 与 gpui_component 的 theme.radius 对齐
pub const RADIUS_SM: f32 = 6.0;
/// 卡片、浮层、下拉面板、次级弹窗
pub const RADIUS_MD: f32 = 8.0;
/// 弹窗（占用面积最大，圆角也最大，视觉层级才成立）
pub const RADIUS_LG: f32 = 12.0;

// ==================== 控件尺寸 ====================
/// 常规控件高度：顶部 URL 栏、表单行、弹窗按钮
pub const CONTROL_H: f32 = 32.0;
/// 紧凑控件高度：面板表格行、响应头行里的输入框/按钮
pub const CONTROL_H_SM: f32 = 28.0;
/// 分段按钮（按钮组内单项）高度
pub const SEGMENT_H: f32 = 22.0;
/// 面板标签页高度
pub const TAB_H: f32 = 36.0;
/// 行内图标按钮正方形边长（列表行的「更多…」「删除」等）
pub const ICON_BTN: f32 = 22.0;

// ==================== 图标 ====================
// 图标以前直接用 gpui_component 的 `Sizable::xsmall()/small()`，那是 rem 档位
// （size_3 = 0.75rem、size_3p5 = 0.875rem）：此刻 rem = 16px 所以正好是 12/14px，
// 但 rem 由 `Root::render` 里的 `window.set_rem_size(theme.font_size)` 决定，
// 主题一改字号，图标就跟着正文缩放 —— 而"图标档位"不该随正文字号漂移。
// 所以这里把两档写成绝对像素常量，图标尺寸只有这一个来源。

/// 图标密集档（12px）：凡是"跟着一行内容走"的图标 —— 列表/表格行、面板与弹窗的
/// 小标题（侧栏标题、节标题、表单标签）、徽章、按钮内部的图标、行内图标按钮
/// （ICON_BTN 22 与 16–24px 微方框，含弹窗标题栏关闭按钮）、状态栏等细行里的图标。
pub const ICON_SIZE_SM: f32 = 12.0;
/// 图标标准档（14px）：凡是"独立占一块地方"的图标 —— 弹窗标题图标、
/// 工具栏/标签栏/侧栏 tab 上的图标按钮（容器 ≥28px）、图标徽章（36px 方框）。
pub const ICON_SIZE_MD: f32 = 14.0;
/// 图标与其相邻文字之间的间距。
///
/// 单独起一个名字而不是直接用 GAP_XS：这是"图标与文字"这一种关系的唯一取值，
/// 全应用只有这里可以改；散落的 `.gap(px(2.0))`/`.gap(GAP_S)` 都是不一致的来源。
pub const ICON_TEXT_GAP: f32 = GAP_XS;

// ==================== 间距 ====================
/// 小间距 / 中间距 / 大间距 / 面板内边距
pub const GAP_XS: f32 = 4.0;
pub const GAP_S: f32 = 8.0;
pub const GAP_M: f32 = 12.0;
pub const GAP_L: f32 = 16.0;
pub const PANEL_PAD: f32 = 12.0;

// ==================== 表单 ====================
/// 表单标签列宽 / 行高（设置、认证等表单类面板统一）
pub const FORM_LABEL_W: f32 = 160.0;
pub const FORM_ROW_H: f32 = CONTROL_H + 6.0;

/// 在两个主题色之间做线性插值：`t = 0` 取 `a`，`t = 1` 取 `b`。
///
/// 派生状态色（hover 比常态重一点、pressed 再重一点）只需要这样一个函数，
/// 不需要为基础色之外的颜色再往主题里加字段，也就不会出现「某套主题忘了填」的情况。
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// 相对亮度（Rec.601 加权），只用来在主题里挑「更暗的那个颜色」，不做任何颜色判定阈值。
fn luminance(c: Rgba) -> f32 {
    0.299 * c.r + 0.587 * c.g + 0.114 * c.b
}

impl Theme {
    /// 悬停底色。
    ///
    /// 统一走 `muted_background`：以前列表行 hover 用 `code_background`，
    /// 而 dracula/gruvbox/tokyonight/latte 这几套主题里两者并不相等，
    /// 于是「侧栏 hover」和「面板按钮 hover」颜色不一样。
    pub fn hover_bg(&self) -> Rgba {
        self.muted_background
    }

    /// 按下 / 选中底色：在 hover 底色上再叠一点前景色，形成 hover → pressed 的轻重差。
    pub fn active_bg(&self) -> Rgba {
        mix(self.muted_background, self.foreground, 0.12)
    }

    /// 主色按钮的悬停色。
    ///
    /// 以前是 `.hover(|s| s.opacity(0.9))`，那是把按钮整体调透明，
    /// 叠在面板底色上会让按钮看起来「褪色」而不是「亮起来」；这里改成偏向前景色混色。
    pub fn accent_hover(&self) -> Rgba {
        mix(self.accent, self.accent_foreground, 0.10)
    }

    /// 主色按钮的按下色，比 hover 再深一档。
    pub fn accent_pressed(&self) -> Rgba {
        mix(self.accent, self.accent_foreground, 0.20)
    }

    /// 焦点边框色。
    ///
    /// 直接用主题主色：组件库默认的焦点环是固定的蓝色，在 gruvbox / monokai /
    /// latte 这类主题下会和整体配色打架。
    pub fn focus_border(&self) -> Rgba {
        self.accent
    }

    /// 可编辑控件（输入框、文本域）的底色。
    ///
    /// 统一走 `input_background`：在 light / latte / tokyonight 里
    /// `code_background` 与 `input_background` 不同，混用会让相邻两个输入框深浅不一。
    pub fn control_bg(&self) -> Rgba {
        self.input_background
    }

    /// 弹窗遮罩。
    ///
    /// 取「背景色与前景色里更暗的那个」再压到 25% 亮度：浅色主题下这得到接近黑的
    /// 深灰（前面是白底，压得下去），深色主题下得到比页面更黑的颜色（页面本身已经很暗，
    /// 只有比它更黑才谈得上「压暗」）。全程由调色板派生，不写死黑色，
    /// 也顺手把以前 0x88/0x66/0x55/0x44 四种不统一的遮罩透明度收敛成一个值。
    pub fn scrim(&self) -> Rgba {
        let base = if luminance(self.background) <= luminance(self.foreground) {
            self.background
        } else {
            self.foreground
        };
        Rgba {
            r: base.r * 0.25,
            g: base.g * 0.25,
            b: base.b * 0.25,
            a: 0.5,
        }
    }
}
