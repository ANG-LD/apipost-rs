//! 主题表的回归测试。
//!
//! 放在独立文件里：直接在 themes/mod.rs 写 #[test] 会触发宏的递归上限
//! （themes/mod.rs 的模块环境比这里复杂得多）。

use crate::ui::components::{
    button_size_for_icon, icon_tone_color, IconTier, IconTone, BUTTON_ICON_RATIO,
};
use crate::ui::themes::component_theme::{component_config, contrast_ratio, readable_on};
use crate::ui::themes::tokens::{GAP_XS, ICON_SIZE_MD, ICON_SIZE_SM, ICON_TEXT_GAP};
use crate::ui::themes::Theme;
use gpui::{rgb, Rgba};
use gpui_component::Size;
use std::collections::HashSet;

// ==================== 组件库高亮主题（编辑器配色）的回归守卫 ====================
//
// 背景：输入框里的 JSON 编辑器是 gpui_component 画的，它的正文底色、行号栏底色、
// 行号颜色、语法高亮全部读 `gpui_component::theme::Theme::highlight_theme`。
// 项目自己的 `ui::Theme` 不管这块，所以桥接层必须显式给一份，否则会停在该库
// 自带的浅色高亮主题上：深色主题里行号栏变成白色（症状 1），JSON 的键/值用的是
// 浅色主题的深蓝深绿，压在深色底上几乎看不见（症状 2）。

// 独立的 WCAG 对比度实现。刻意不复用桥接层的 `contrast_ratio`：
// 共用同一份计算函数只能验证自洽，独立算一遍才拦得住「映射结果本身不够读」。
fn linearize(channel: f32) -> f32 {
    let c = channel.clamp(0.0, 1.0);
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: Rgba) -> f32 {
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn hex(color: Rgba) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (color.r * 255.0) as u32,
        (color.g * 255.0) as u32,
        (color.b * 255.0) as u32
    )
}

/// 从桥接层**真正交给组件库的那份配置**里读回一个颜色。
///
/// 走 `component_config` → `highlight` → 序列化回 Zed 主题格式的 JSON 再取值：
/// 字段名写错、漏赋值、给错结构，都会在这里暴露；如果测试自己重算一遍颜色，
/// 这类错误一个也拦不住。
fn mapped_color(theme: &Theme, pointer: &str) -> Rgba {
    let config = component_config(theme);
    let style = config
        .highlight
        .as_ref()
        .expect("必须给组件库高亮主题：留空会停在该库自带的浅色主题上");
    let value = serde_json::to_value(style).expect("高亮主题可序列化");
    let raw = value
        .pointer(pointer)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("{} 没有映射出颜色", pointer));
    Rgba::try_from(raw).unwrap_or_else(|e| panic!("{} = {} 不是合法颜色: {}", pointer, raw, e))
}

/// JSON 语义角色 → 高亮主题字段。指针按 Zed 主题（= 组件库）的 JSON 结构写。
const JSON_ROLES: [(&str, &str); 6] = [
    ("/syntax/property/color", "json_key"),
    ("/syntax/string/color", "json_string"),
    ("/syntax/number/color", "json_number"),
    ("/syntax/boolean/color", "json_boolean"),
    ("/syntax/constant/color", "json_null"),
    ("/syntax/punctuation/color", "json_bracket"),
];

/// 症状 1 守卫：行号栏底色不能发白，必须与编辑器底色属于同一明暗体系。
#[test]
fn gutter_background_matches_the_editor_background_family() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let editor_bg = mapped_color(&theme, "/editor.background");
        let gutter = mapped_color(&theme, "/editor.gutter.background");

        // 正文底必须等于 code_editor_view 传给 `Input::bg()` 的 code_background，
        // 否则行号栏和正文之间会露出色差缝
        assert_eq!(
            editor_bg, theme.code_background,
            "{} 的编辑器底色应当取调色板的 code_background",
            name
        );
        // 白色行号栏就是这个回归本身
        assert_ne!(hex(gutter), "#ffffff", "{} 的行号栏底色变成了白色", name);
        // 同一明暗体系：行号栏只是编辑器底色上很淡的一档，
        // 对比度超过 1.3 就会看成两条不同的色块
        let gutter_gap = contrast(gutter, editor_bg);
        assert!(
            gutter_gap <= 1.3,
            "{} 的行号栏 {} 与编辑器底 {} 差得太多（对比度 {:.3}）",
            name,
            hex(gutter),
            hex(editor_bg),
            gutter_gap
        );
        if Theme::is_light(name) {
            assert!(
                relative_luminance(gutter) > 0.5,
                "{} 是浅色主题，行号栏 {} 不该是暗色",
                name,
                hex(gutter)
            );
        } else {
            assert!(
                relative_luminance(gutter) < 0.5,
                "{} 是深色主题，行号栏 {} 不该发白（亮度 {:.3}）",
                name,
                hex(gutter),
                relative_luminance(gutter)
            );
        }
    }
}

/// 症状 2 守卫：JSON 语法色压在编辑器底色上必须够读（WCAG ≥ 3.0，12 套主题逐套断言）。
#[test]
fn json_syntax_colors_are_readable_on_the_editor_background() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let editor_bg = mapped_color(&theme, "/editor.background");
        for (pointer, palette_field) in JSON_ROLES {
            let color = mapped_color(&theme, pointer);
            let ratio = contrast(color, editor_bg);
            assert!(
                ratio >= 3.0,
                "{} 的 {}（调色板 {}）映射成 {} 后，压在编辑器底 {} 上对比度只有 {:.3}，看不清",
                name,
                pointer,
                palette_field,
                hex(color),
                hex(editor_bg),
                ratio
            );
        }
    }
}

/// 行号 / 当前行也必须可读：行号压在行号栏底色上，当前行号压在当前行底色上。
#[test]
fn line_numbers_are_readable_on_the_gutter_and_active_line() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let gutter = mapped_color(&theme, "/editor.gutter.background");
        let line_number = mapped_color(&theme, "/editor.line_number");
        let active_line_bg = mapped_color(&theme, "/editor.active_line.background");
        let active_line_number = mapped_color(&theme, "/editor.active_line_number");

        let on_gutter = contrast(line_number, gutter);
        assert!(
            on_gutter >= 3.0,
            "{} 的行号 {} 压在行号栏 {} 上对比度只有 {:.3}",
            name,
            hex(line_number),
            hex(gutter),
            on_gutter
        );
        let on_active = contrast(active_line_number, active_line_bg);
        assert!(
            on_active >= 3.0,
            "{} 的当前行号 {} 压在当前行底色 {} 上对比度只有 {:.3}",
            name,
            hex(active_line_number),
            hex(active_line_bg),
            on_active
        );
        // 当前行只能是编辑器底色的淡档：太重的底色会把文字压没
        let active_gap = contrast(active_line_bg, mapped_color(&theme, "/editor.background"));
        assert!(
            active_gap <= 1.3,
            "{} 的当前行底色 {} 太重（相对编辑器底色对比度 {:.3}）",
            name,
            hex(active_line_bg),
            active_gap
        );
    }
}

/// 每个语法角色都必须取自调色板里对应的那个字段（而不是写死颜色或串了字段）。
#[test]
fn mapped_json_colors_derive_from_the_matching_palette_fields() {
    let mut key_colors = HashSet::new();
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let bg = mapped_color(&theme, "/editor.background");
        let fg = mapped_color(&theme, "/editor.foreground");
        let palette = [
            (theme.json_key, "/syntax/property/color"),
            (theme.json_string, "/syntax/string/color"),
            (theme.json_number, "/syntax/number/color"),
            (theme.json_boolean, "/syntax/boolean/color"),
            (theme.json_null, "/syntax/constant/color"),
            (theme.json_bracket, "/syntax/punctuation/color"),
        ];
        for (palette_color, pointer) in palette {
            assert_eq!(
                mapped_color(&theme, pointer),
                readable_on(palette_color, bg, fg),
                "{} 的 {} 不是由对应调色板色 {} 派生出来的",
                name,
                pointer,
                hex(palette_color)
            );
        }
        key_colors.insert(hex(mapped_color(&theme, "/syntax/property/color")));
    }
    // 12 套主题必须给出彼此不同的键色：只有一份颜色表才可能这么多样
    assert!(
        key_colors.len() >= 10,
        "键色只有 {} 种，看起来像写死的颜色",
        key_colors.len()
    );
}

/// 桥接层的兜底不能越界：已经够读的颜色要保持原样，不够读的必须被提到达标。
#[test]
fn contrast_floor_only_touches_colors_that_need_it() {
    let bg = rgb(0x18182a);
    let fg = rgb(0xe8e8f0);
    // 对比度足够 → 原样返回，不引入无谓的偏色
    let good = rgb(0x60a5fa);
    assert_eq!(readable_on(good, bg, fg), good);
    // 对比度不足 → 混到达标，且只朝前景色方向混（不会跑到别的色相上）
    let poor = rgb(0x101425);
    let fixed = readable_on(poor, bg, fg);
    assert!(contrast_ratio(fixed, bg) >= 3.0, "兜底后仍不达标");
    assert!(
        contrast_ratio(fixed, bg) > contrast_ratio(poor, bg),
        "兜底没有改善对比度"
    );
    for (original, target, result) in [
        (poor.r, fg.r, fixed.r),
        (poor.g, fg.g, fixed.g),
        (poor.b, fg.b, fixed.b),
    ] {
        let (lo, hi) = if original <= target {
            (original, target)
        } else {
            (target, original)
        };
        assert!(
            result >= lo - 1.0 / 255.0 && result <= hi + 1.0 / 255.0,
            "兜底把通道混出了原始色与前景色之间"
        );
    }
}



/// 主题名 → 颜色表必须一一对上，避免 from_str 漏写分支或颜色写错
#[test]
fn every_theme_name_resolves_its_own_palette() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        assert_eq!(&theme.name, name, "from_str({}) 返回了 {} 的调色板", name, theme.name);
    }
}

/// 新增的四套主题：底色/前景/accent 抽查（改色时这条会挡住误改）
#[test]
fn new_themes_have_expected_key_colors() {
    let cases = [
        ("dracula", 0x282a36, 0xf8f8f2, 0xbd93f9),
        ("tokyonight", 0x1a1b26, 0xc0caf5, 0x7aa2f7),
        ("gruvbox", 0x282828, 0xebdbb2, 0xfabd2f),
        ("latte", 0xeff1f5, 0x4c4f69, 0x7287fd),
    ];
    for (name, bg, fg, accent) in cases {
        let t = Theme::from_str(name);
        assert_eq!(t.background, rgb(bg), "{} 背景色不对", name);
        assert_eq!(t.foreground, rgb(fg), "{} 前景色不对", name);
        assert_eq!(t.accent, rgb(accent), "{} 强调色不对", name);
    }
}

/// 浅色主题要能被识别出来，否则组件库会按 Dark 模式渲染，对比度会出问题
#[test]
fn light_theme_detection_covers_new_light_theme() {
    assert!(Theme::is_light("light"));
    assert!(Theme::is_light("latte"), "latte 是浅色主题，必须走 Light 模式");
    for dark in ["dark", "dracula", "tokyonight", "gruvbox", "nord", "monokai"] {
        assert!(!Theme::is_light(dark), "{} 是深色主题", dark);
    }
}

/// Ctrl+T 循环依赖 NAMES：顺序里不能有重复，且必须覆盖 from_str 的所有分支
#[test]
fn theme_names_are_unique_and_cycle_safe() {
    let mut sorted = Theme::NAMES.to_vec();
    sorted.sort_unstable();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(sorted.len(), before, "NAMES 里有重复项，Ctrl+T 会跳过主题");
    assert!(before >= 12, "主题数量异常: {}", before);
}

// ==================== 图标一致性规则 ====================
//
// 全应用 47 处图标调用都走 `components::themed_icon`，它的尺寸来自 IconTier、
// 颜色来自 icon_tone_color。所以只要下面两张表被钉住：
//   * 不可能再出现"没写尺寸的图标"（尺寸只有两档，且都是绝对值，不随 rem 漂移）；
//   * 不可能再出现"同一个语义在不同面板里颜色不同"。
// 这两条正是本轮改造的目标，用测试把它们钉死，而不是靠注释。

/// 尺寸只有两档：12（密集行内/徽章/按钮内）与 14（独立图标/弹窗标题/工具栏）。
#[test]
fn icon_size_has_exactly_two_absolute_tiers() {
    assert_eq!(IconTier::Dense.px(), ICON_SIZE_SM);
    assert_eq!(IconTier::Regular.px(), ICON_SIZE_MD);
    assert_eq!(ICON_SIZE_SM, 12.0, "密集档就是 12px");
    assert_eq!(ICON_SIZE_MD, 14.0, "标准档就是 14px");
    assert!(
        ICON_SIZE_SM < ICON_SIZE_MD,
        "两档必须有大小差别，否则分档没有意义"
    );
    // 图标与文字的间距只有一处定义，别的文件不许再写 2.0/8.0 之类的数字
    assert_eq!(ICON_TEXT_GAP, GAP_XS);
    assert_eq!(ICON_TEXT_GAP, 4.0);
}

/// 组件库 `Button` 里的图标也必须落在同样的两档上。
///
/// `gpui-component` 的 `Button` 用**自身** size 换算出图标大小
/// （`RenderOnce for Button`：`icon_size = size * 0.75`，且会覆盖调用方传给
/// `ButtonIcon` 的尺寸），所以调用点必须用 `button_size_for_icon(tier)` 反解。
/// 这条测试把该换算钉住：组件库一旦换算法、或有人改回 `.xsmall()/.small()`
/// （rem 档位，会随主题字号漂移），这里立刻报出来。
#[test]
fn library_button_icons_land_on_the_same_two_tiers() {
    for (tier, expected) in [
        (IconTier::Dense, ICON_SIZE_SM),
        (IconTier::Regular, ICON_SIZE_MD),
    ] {
        match button_size_for_icon(tier) {
            Size::Size(button_size) => {
                let icon_size = f32::from(button_size) * BUTTON_ICON_RATIO;
                assert_eq!(
                    icon_size, expected,
                    "{:?} 档经组件库 Button 换算出的图标是 {}px，不是 {}px",
                    tier, icon_size, expected
                );
            }
            other => panic!(
                "{:?} 必须给绝对像素（Size::Size），rem 档位 {:?} 会随主题字号漂移",
                tier, other
            ),
        }
    }
}

/// 语义色必须取到它名字所对应的那个调色板字段（12 套主题逐套断言）。
///
/// 这条测试的价值在于：改 `icon_tone_color` 的人如果顺手把 Muted 指向了 `border`、
/// 或把 Danger 指向了 `accent`，12 套主题里立刻会有一套断言失败。
#[test]
fn icon_tone_maps_to_the_palette_field_it_names() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let cases = [
            (IconTone::Muted, theme.muted_foreground, "muted_foreground"),
            (IconTone::Primary, theme.foreground, "foreground"),
            (IconTone::Accent, theme.accent, "accent"),
            (IconTone::Danger, theme.error, "error"),
            (IconTone::Success, theme.success, "success"),
        ];
        for (tone, expected, field) in cases {
            let got = icon_tone_color(tone, &theme)
                .unwrap_or_else(|| panic!("{} 的 {:?} 不该是 Inherit", name, tone));
            assert_eq!(
                got, expected,
                "{} 的 {:?} 必须取调色板的 {}",
                name, tone, field
            );
        }
        // Inherit 是"刻意不写颜色"，让 Icon 去读窗口文字色 —— 必须返回 None，
        // 否则容器 hover/选中时图标不会跟着换色
        assert!(
            icon_tone_color(IconTone::Inherit, &theme).is_none(),
            "{} 的 Inherit 不能返回具体颜色",
            name
        );
    }
}

/// 次级图标（Muted）是界面上出现最多的一档（列表箭头、删除图标、状态栏图标），
/// 压在页面底色上必须看得见。
///
/// 门槛 3.0 = WCAG 对非文本图形/图标的最低要求（本轮之前是 2.5 的"锁住现状"）。
/// 实测 12 套主题（改色后，全部达标）：
///   dark 6.16 / ocean 5.89 / sunset 5.56 / sepia 4.91 / light 4.83 /
///   nord 4.64 / forest 4.48 / gruvbox 4.02 / monokai 3.03 / dracula 3.03 /
///   tokyonight 3.28 / latte 3.02
/// tokyonight 与 latte 是这轮唯一动过的两套调色板（只提亮 muted_foreground 一个数值）：
/// 前者原值 #565f89 = 2.76，后者原值 #8c8fa1 = 2.83 —— 都不足 3.0，只靠门槛是拦不住的。
/// 这条测试从此不许再退化：谁把这档色调暗、或把配色改回旧值，就会报出来。
#[test]
fn muted_icons_stay_readable_on_every_background() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let ratio = contrast(theme.muted_foreground, theme.background);
        assert!(
            ratio >= 3.0,
            "{} 的次级图标色 {} 压在底色 {} 上对比度只有 {:.2}，低于 WCAG 非文本图标的 3.0",
            name,
            hex(theme.muted_foreground),
            hex(theme.background),
            ratio
        );
        // 达标不能靠"把次级色提亮到接近正文色"：Muted 与 Primary(foreground) 还必须
        // 保持可区分的差距（12 套实测最小 2.17，门槛取 1.8 留出余量）
        let vs_foreground = contrast(theme.muted_foreground, theme.foreground);
        assert!(
            vs_foreground >= 1.8,
            "{} 的次级图标色 {} 已经贴近前景色 {}（对比度只有 {:.2}），失去了'次级'语义",
            name,
            hex(theme.muted_foreground),
            hex(theme.foreground),
            vs_foreground
        );
    }
}


#[test]
fn tab_scroll_arrows_enable_in_the_right_directions() {
    // 回归：`ScrollHandle::offset()` 向右滚动时为负值，且只允许在
    // `tab_scroll_offset_from_handle` 里翻一次符号 —— 外部一律用非负的"已向右滚动距离"比较。
    // 早期实现翻了两次（渲染处取反一次、纯函数里又取反一次，等于没翻），
    // 于是 can_left 恒为假：「左箭头永远置灰不可点、右箭头永远可点」。
    use super::main_view::{tab_scroll_button_states, tab_scroll_offset_from_handle};
    // 第一步：负值 offset → 非负的"已向右滚动距离"
    assert_eq!(tab_scroll_offset_from_handle(0.0), 0.0);
    assert_eq!(tab_scroll_offset_from_handle(-150.0), 150.0);
    assert_eq!(tab_scroll_offset_from_handle(-0.4), 0.4);
    // 防御：gpui 只会在 [-max, 0] 里写 offset；万一拿到正值，一律当"未滚动"，
    // 不能让它把左箭头误判成可用
    assert_eq!(tab_scroll_offset_from_handle(7.0), 0.0);
    // 第二步：由"已滚动距离"判定两侧箭头
    assert_eq!(tab_scroll_button_states(0.0, 300.0), (false, true)); // 最左：只能向右
    assert_eq!(tab_scroll_button_states(150.0, 300.0), (true, true)); // 中间：两边都可用
    assert_eq!(tab_scroll_button_states(300.0, 300.0), (true, false)); // 最右：只能向左
    assert_eq!(tab_scroll_button_states(0.0, 0.0), (false, false)); // 标签未溢出：都禁用
    // 边界：可滚动范围小到不足 1px 时，方向判定两侧都应用同一个 0.5px 容差，
    // 否则会出现「两个箭头同时亮」这种自相矛盾的状态
    assert_eq!(tab_scroll_button_states(0.0, 0.4), (false, false));
    assert_eq!(tab_scroll_button_states(0.4, 0.4), (false, false));
    // 半像素抖动不该让箭头闪烁
    assert_eq!(tab_scroll_button_states(0.49, 300.0), (false, true));
    // 弹性越界（gpui 会把 offset 夹回区间内）也不能把「已滚到最右」判成还能继续右滚
    assert_eq!(tab_scroll_button_states(300.0, 300.0).1, false);
}

#[test]
fn tab_scroll_left_arrow_is_clickable_when_active_tab_is_rightmost() {
    // 用户报的场景：**激活的标签在最右侧**（标签条已滚到最右）时，左箭头必须可用可点。
    // 这里把整条链路（gpui 原始 offset → 已滚动距离 → 箭头状态）都钉住，
    // 让"渲染处已取反 + 纯函数又取反"的双重取负错误无法再溜过去。
    use super::main_view::{
        tab_scroll_button_states, tab_scroll_limit, tab_scroll_offset_from_handle,
    };
    // 1200 宽窗口：标签条可视宽 836，7 个定宽标签内容宽 =(150+1)*7-1 = 1056
    let max = tab_scroll_limit(151.0 * 7.0 - 1.0, 836.0);
    assert_eq!(max, 220.0);
    // 滚到最右时 gpui 写回的原始 offset 是 -220 → 两个箭头状态必须是（左可用、右置灰）
    let scrolled = tab_scroll_offset_from_handle(-max);
    assert_eq!(scrolled, 220.0);
    assert_eq!(tab_scroll_button_states(scrolled, max), (true, false));
    // 停在最左时仍是（左置灰、右可用）—— 不能矫枉过正成"左箭头永远可点"
    assert_eq!(tab_scroll_button_states(tab_scroll_offset_from_handle(0.0), max), (false, true));
    // 反向走一屏（-0.49 是浮点抖动）：不构成"已滚动"，左箭头保持置灰
    assert_eq!(tab_scroll_button_states(tab_scroll_offset_from_handle(-0.49), max), (false, true));
}

#[test]
fn tab_scroll_limit_covers_viewport_edge_cases() {
    // 回归：可视宽度以前来自一个「只在构造时写过 0.0」的字段，等于视口恒为 0。
    // 于是"内容宽 - 视口"退化成"整个内容宽"，右箭头恒可点、左箭头恒灰。
    use super::main_view::tab_scroll_limit;
    // 未溢出：内容比视口窄
    assert_eq!(tab_scroll_limit(600.0, 1000.0), 0.0);
    // 内容宽刚好等于可视宽：边界上不可滚动
    assert_eq!(tab_scroll_limit(1000.0, 1000.0), 0.0);
    // 内容宽略大于可视宽：可滚动，且范围就是差值
    assert_eq!(tab_scroll_limit(1000.5, 1000.0), 0.5);
    assert_eq!(tab_scroll_limit(1149.0, 1000.0), 149.0);
    // 一个标签都没有（内容宽为 0）也不该算出负的可滚动范围
    assert_eq!(tab_scroll_limit(0.0, 1000.0), 0.0);
    // 退化输入：视口宽度还没测出来（首次 prepaint 之前），必须当作「不可滚动」。
    // 若按"视口 = 0"去算，会得到等于整个内容宽的虚高范围 —— 那正是本次 bug
    assert_eq!(tab_scroll_limit(1207.0, 0.0), 0.0);
    assert_eq!(tab_scroll_limit(1207.0, -1.0), 0.0);
    // 内容宽为 0 且视口未知时同样是 0（不出现 NaN / 负数）
    assert_eq!(tab_scroll_limit(0.0, 0.0), 0.0);
}

#[test]
fn tab_scroll_click_enables_the_left_arrow() {
    // 用户可见行为的纯函数复现：点一次右箭头之后，左箭头必须变成可用且能点。
    // 这条链路是 tab_scroll_step_offset（点一次算新偏移）→ tab_scroll_button_states
    // （由新偏移决定箭头状态），任何一段的符号/夹紧写错都会在这里暴露。
    // 注意偏移量统一用非负数（"已向右滚动的距离"），符号只在
    // tab_scroll_offset_from_handle 里翻一次 —— 否则就是双重取负那个老 bug。
    use super::main_view::{tab_scroll_button_states, tab_scroll_limit, tab_scroll_step_offset};

    // 可视宽 1000，9 个定宽标签：内容宽 = (150+1)*9-1 = 1358，可滚动范围 358
    let max = tab_scroll_limit(1358.0, 1000.0);
    assert_eq!(max, 358.0);

    // 最左：左箭头不可点
    let mut offset = 0.0;
    assert_eq!(tab_scroll_button_states(offset, max), (false, true));

    // 点一次右箭头 → 左箭头立刻可用（步进 = 标签宽 150 + 间距 1）
    offset = tab_scroll_step_offset(offset, 1, max);
    assert_eq!(offset, 151.0);
    assert_eq!(tab_scroll_button_states(offset, max), (true, true));

    // 继续点到超过最大范围：夹在 max 上，右箭头置灰、左箭头仍可用
    offset = tab_scroll_step_offset(offset, 5, max);
    assert_eq!(offset, max);
    assert_eq!(tab_scroll_button_states(offset, max), (true, false));

    // 在最右端点一次左箭头：回到上一格（358/151 先对齐到第 2 格，再退 1 格），
    // 右箭头重新可用
    offset = tab_scroll_step_offset(offset, -1, max);
    assert_eq!(offset, 151.0);
    assert_eq!(tab_scroll_button_states(offset, max), (true, true));

    // 一路点回最左：不会越过 0，左箭头重新置灰
    offset = tab_scroll_step_offset(offset, -3, max);
    assert_eq!(offset, 0.0);
    assert_eq!(tab_scroll_button_states(offset, max), (false, true));

    // 未溢出（内容宽比可视宽窄）：两个方向都不可滚动，点也不会产生偏移
    let no_max = tab_scroll_limit(600.0, 1000.0);
    assert_eq!(no_max, 0.0);
    let stuck = tab_scroll_step_offset(0.0, 1, no_max);
    assert_eq!(stuck, 0.0);
    assert_eq!(tab_scroll_button_states(stuck, no_max), (false, false));

    // 视口未知（首次 prepaint 之前）：同样不可滚动，不会出现"能点但滚不动"的箭头
    let unknown_max = tab_scroll_limit(1358.0, 0.0);
    assert_eq!(tab_scroll_step_offset(0.0, 1, unknown_max), 0.0);
    assert_eq!(
        tab_scroll_button_states(0.0, unknown_max),
        (false, false)
    );
}


// ==================== 可点击控件的反馈色规则 ====================
//
// 背景：界面里有若干「看着能点、点了没反应」的容器 —— 行内 ✓/○ 开关、文件选择框、
// 侧栏 tab、预览区的「在浏览器打开」按钮等。补反馈时有两类坑，都用测试钉住：
//   1. gpui 的 `.hover()` / `.active()` 只在元素带 `global_id`（= `.id()`）时才参与
//      样式计算，且同层共用一个 id 会共享 element state（悬停一行、全部高亮）；
//   2. 反馈底色压着前景色，颜色配错会把文字吃掉（尤其 ✓ 的 success 与 ○ 的 muted）。

/// 行内 ✓/○ 开关的前景色来源：✓ = success，○ = muted_foreground。
///
/// 面板里的开关与这条断言读的是**同一个函数**（`components::toggle_chip_foreground`），
/// 所以"改成别的语义色"会立刻在这里暴露。
#[test]
fn toggle_chip_foreground_follows_the_state() {
    use crate::ui::components::toggle_chip_foreground;
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        assert_eq!(
            toggle_chip_foreground(true, &theme),
            theme.success,
            "{} 的 ✓ 应当取 success",
            name
        );
        assert_eq!(
            toggle_chip_foreground(false, &theme),
            theme.muted_foreground,
            "{} 的 ○ 应当取 muted_foreground",
            name
        );
    }
}


/// hover / 按下时两种状态用同一个前景色，且必须比常态更清楚（不能"补反馈把字吃掉"）。
///
/// 为什么统一换前景色而不是各自保留 success / muted：反馈底色是 muted_background 系，
/// 实测 `muted_foreground × active_bg` 最低只有 1.40（dracula）、success × active_bg
/// 最低 2.31（latte），保留状态色就会把字吃掉；换成 `foreground` 后同一批底色上
/// 最低仍有 6.57（latte，见测试打印）。状态由字形（✓ / ○）表达 —— 在
/// light/sepia/latte 里两种状态色的对比本来就只有 1.0~1.3，从来不是靠颜色区分的。
///
/// 门槛 4.5 = WCAG AA 对正文的要求（✓/○ 是 12px 字形）。
/// 备注（既有配色，本轮不动）：latte 的 success 压在面板底色上只有 2.96，
/// 那不是反馈引入的问题，而是该主题状态色本身的对比度。
#[test]
fn toggle_chip_feedback_foreground_is_the_most_readable_one() {
    use crate::ui::components::{toggle_chip_foreground, toggle_chip_hover_foreground};
    let mut worst = f32::MAX;
    let mut worst_where = String::new();
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        let feedback = toggle_chip_hover_foreground(&theme);
        for (state, fg) in [
            ("✓ 启用", toggle_chip_foreground(true, &theme)),
            ("○ 禁用", toggle_chip_foreground(false, &theme)),
        ] {
            for (bg_name, bg) in [("hover", theme.hover_bg()), ("按下", theme.active_bg())] {
                let ratio = contrast(feedback, bg);
                if ratio < worst {
                    worst = ratio;
                    worst_where = format!("{} / {} / {}", name, state, bg_name);
                }
                assert!(
                    ratio >= 4.5,
                    "{} 的 {} 压在 {} 底色 {} 上对比度只有 {:.2}",
                    name,
                    state,
                    bg_name,
                    hex(bg),
                    ratio
                );
            }
        }
    }
    println!("开关在反馈底色上的最差对比度：{:.2}（{}）", worst, worst_where);
}

/// 表单数据行 true / false 值切换 chip：加了 hover / 按下反馈后仍然够读。
///
/// 底色是状态色按 alpha 叠在面板底色上（chip 的父容器就是面板 → `theme.background`）。
/// 浓度越高，底色越靠近状态色：若文字仍是状态色就会越描越糊（实测最低 1.96），
/// 所以反馈态把文字换成 `foreground` —— 同一批底色上最低 4.07（latte 的 false 按下）。
/// 门槛取 4.0（略低于 AA 的 4.5：这种 chip 是"按下瞬间"的临时态）。
#[test]
fn value_chip_feedback_keeps_the_label_readable() {
    use crate::ui::components::{
        VALUE_CHIP_BASE_ALPHA, VALUE_CHIP_HOVER_ALPHA, VALUE_CHIP_PRESSED_ALPHA,
    };
    use crate::ui::themes::tokens::mix;
    assert!(
        VALUE_CHIP_BASE_ALPHA < VALUE_CHIP_HOVER_ALPHA
            && VALUE_CHIP_HOVER_ALPHA < VALUE_CHIP_PRESSED_ALPHA,
        "反馈档必须比常态更浓、按下比 hover 更浓，否则看不出变化"
    );
    let mut worst = f32::MAX;
    let mut worst_where = String::new();
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        for (state, color) in [("true", theme.success), ("false", theme.error)] {
            // 常态：文字与底色同色（这是改造前就有的写法，保持不变）
            let at_rest = contrast(
                color,
                mix(theme.background, color, VALUE_CHIP_BASE_ALPHA),
            );
            for (label, alpha) in [
                ("hover", VALUE_CHIP_HOVER_ALPHA),
                ("按下", VALUE_CHIP_PRESSED_ALPHA),
            ] {
                // 反馈态：文字换成正文色（面板里就是这么写的）
                let ratio = contrast(
                    theme.foreground,
                    mix(theme.background, color, alpha),
                );
                if ratio < worst {
                    worst = ratio;
                    worst_where = format!("{} / {} / {}", name, state, label);
                }
                assert!(
                    ratio >= 4.0,
                    "{} 的 {} chip 在 {} 底色下文字对比度只有 {:.2}（常态 {:.2}）",
                    name,
                    state,
                    label,
                    ratio,
                    at_rest
                );
            }
        }
    }
    println!("true/false chip 在反馈底色上的最差对比度：{:.2}（{}）", worst, worst_where);
}

/// hover / 按下的派生色必须"比常态重一档"，且只朝前景色方向混（不跑到别的色相上）。
///
/// `toggle_switch` 的轨道常态是语义色（success / border），反馈色由 `tint_hover()` /
/// `tint_active()` 派生 —— 这两条保证"看得出变化"且"颜色仍然来自主题"。
#[test]
fn tint_hover_and_active_deepen_toward_the_foreground() {
    for name in Theme::NAMES {
        let theme = Theme::from_str(name);
        for base in [theme.success, theme.border, theme.accent] {
            let hover = theme.tint_hover(base);
            let active = theme.tint_active(base);
            assert_ne!(hover, base, "{}：hover 色必须与常态不同", name);
            assert_ne!(active, hover, "{}：按下色必须比 hover 再重一档", name);
            for (original, target, result) in [
                (base.r, theme.foreground.r, hover.r),
                (base.g, theme.foreground.g, hover.g),
                (base.b, theme.foreground.b, hover.b),
            ] {
                let (lo, hi) = if original <= target {
                    (original, target)
                } else {
                    (target, original)
                };
                assert!(
                    result >= lo - 1.0 / 255.0 && result <= hi + 1.0 / 255.0,
                    "{}：派生色把通道混到了底色与前景色之间以外",
                    name
                );
            }
        }
    }
}

/// 列表行里的可点击控件，id 必须带上该行的键（下标 / 参数名）。
///
/// gpui 的 `.hover()` / `.active()` 只在元素有 `global_id`（即 `.id()`）时才参与样式
/// 计算；而同一层里多个元素共用同一个 id 会共享同一份 element state ——
/// 表现就是「悬停一行、所有行一起高亮」。这四处正是本轮补反馈的地方，
/// 用 `include_str!` 把"id 里必须带 `{}` 行键"钉住：谁把 `{}` 去掉，这里立刻失败。
#[test]
fn per_row_click_targets_keep_row_scoped_ids() {
    let params = include_str!("request/params_panel.rs");
    let headers = include_str!("request/headers_panel.rs");
    let body = include_str!("request/body_panel.rs");

    for (file, label, pattern) in [
        (params, "参数行 ✓/○ 开关", "param-toggle-{}"),
        (headers, "请求头行 ✓/○ 开关", "header-toggle-{}"),
        (body, "表单数据行 ✓/○ 开关", "{}-toggle-{}"),
        (body, "表单数据行 true/false 值切换", "{}-bool-{}"),
        (body, "表单数据行文件选择框", "{}-file-{}"),
    ] {
        assert!(
            file.contains(&format!("format!(\"{}\"", pattern)),
            "{label} 的 id 必须由 format! 生成并带行键：{pattern}"
        );
    }
    // 三个面板的行内开关必须走同一份实现（尺寸/颜色/hover 规则只写一遍）
    assert!(params.contains("enabled_toggle("), "参数面板应当使用 components::enabled_toggle");
    assert!(headers.contains("enabled_toggle("), "请求头面板应当使用 components::enabled_toggle");
    assert!(body.contains("enabled_toggle("), "表单数据面板应当使用 components::enabled_toggle");
}

#[test]
fn response_time_switches_unit_at_one_second() {
    // 用户口径：<1s 用毫秒；>=1s 用秒，且**只有带小数时**才保留 2 位小数。
    // 断言逐个写完整字符串（不用 contains）：`1.10 s` 丢掉末尾 0、整数秒被补成 `2.00 s`、
    // 或 60s 以上被"优化"成 `1m 5s` 这类改动，都必须在测试里立刻挂掉。
    use super::main_view::format_response_time;

    // 1s 以内：毫秒，不换算
    assert_eq!(format_response_time(0), "0 ms");
    assert_eq!(format_response_time(1), "1 ms");
    assert_eq!(format_response_time(999), "999 ms");

    // 正好/四舍五入后是整数秒：只写整数，不补 `.00`（1999ms 见下）
    assert_eq!(format_response_time(1000), "1 s");
    assert_eq!(format_response_time(2000), "2 s");
    assert_eq!(format_response_time(65000), "65 s"); // 60s 以上仍用秒，不做分/秒混排

    // 1s 以上且带小数：固定 2 位（`1.10 s` / `1.01 s` 末尾的 0 必须保留）
    assert_eq!(format_response_time(1005), "1.01 s");
    assert_eq!(format_response_time(1100), "1.10 s");
    assert_eq!(format_response_time(1234), "1.23 s");
    assert_eq!(format_response_time(65430), "65.43 s");

    // 关键回归：四舍五入到 2 位之后才变成整数秒的，也要走整数档
    // （1999ms = 1.999s：既不是 `1.99 s`，也不是 `2.00 s`）
    assert_eq!(format_response_time(1999), "2 s");

    // 刻度边界：四舍五入到百分之一秒（1.004s 舍成 1.00、1.995s 进成 2.00），
    // 不是直接截断 —— 截断会把 1995ms 显示成 `1.99 s`。
    assert_eq!(format_response_time(1004), "1 s"); // 1.004s -> 1.00 -> 整数档
    assert_eq!(format_response_time(1994), "1.99 s");
    assert_eq!(format_response_time(1995), "2 s"); // 1.995s -> 2.00 -> 整数档

    // 防御：耗时不该为负，真拿到负值也按 0 显示，界面上不能出现 `-5 ms`
    assert_eq!(format_response_time(-5), "0 ms");
}


