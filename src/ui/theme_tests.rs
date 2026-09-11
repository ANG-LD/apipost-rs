//! 主题表的回归测试。
//!
//! 放在独立文件里：直接在 themes/mod.rs 写 #[test] 会触发宏的递归上限
//! （themes/mod.rs 的模块环境比这里复杂得多）。

use crate::ui::themes::Theme;
use gpui::rgb;



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
