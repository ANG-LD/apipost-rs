//! 主题模块
//!
//! 定义应用的主题色彩方案

use gpui::*;

// 设计令牌（尺寸常量 + 由调色板派生的语义色）与组件库主题桥接。
// tokens 里的常量不在这里 re-export，统一由 components 模块对外提供（只有一条路径，
// 避免 ui/mod.rs 的两个 glob 导入撞名）；`impl Theme` 上的语义色方法无需导入即可调用。
pub mod component_theme;
pub mod tokens;

pub use component_theme::*;

/// 浅色主题
/// 浅色主题 — 简洁清爽
/// 浅色主题 — 轻盈明亮，专业感
pub fn light_theme() -> Theme {
    Theme {
        name: "light".to_string(),
        background: rgb(0xffffff),
        foreground: rgb(0x111827),
        muted_foreground: rgb(0x6b7280),
        accent: rgb(0x2563eb),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0xffffff),
        border: rgb(0xe5e7eb),
        muted_background: rgb(0xf3f4f6),
        sidebar_background: rgb(0xf9fafb),
        code_background: rgb(0xf3f4f6),
        success: rgb(0x059669),
        warning: rgb(0xd97706),
        error: rgb(0xdc2626),
        json_key: rgb(0x2563eb),
        json_string: rgb(0x059669),
        json_number: rgb(0xd97706),
        json_boolean: rgb(0x7c3aed),
        json_null: rgb(0x9ca3af),
        json_bracket: rgb(0x374151),
    }
}

/// 深色主题
/// 深色主题 — 现代暗黑
pub fn dark_theme() -> Theme {
    Theme {
        name: "dark".to_string(),
        background: rgb(0x0c0c1d),
        foreground: rgb(0xe8e8f0),
        muted_foreground: rgb(0x8892a4),
        accent: rgb(0x6366f1),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0x18182a),
        border: rgb(0x2a2a40),
        muted_background: rgb(0x18182a),
        sidebar_background: rgb(0x0f0f20),
        code_background: rgb(0x18182a),
        success: rgb(0x34d399),
        warning: rgb(0xfbbf24),
        error: rgb(0xf87171),
        json_key: rgb(0x60a5fa),
        json_string: rgb(0xf59e0b),
        json_number: rgb(0x4ade80),
        json_boolean: rgb(0xc084fc),
        json_null: rgb(0x6b7280),
        json_bracket: rgb(0xfacc15),
    }
}

/// 扩展Theme结构以包含更多颜色
#[derive(Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub background: gpui::Rgba,
    pub foreground: gpui::Rgba,
    pub muted_foreground: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub accent_foreground: gpui::Rgba,
    pub input_background: gpui::Rgba,
    pub border: gpui::Rgba,
    pub muted_background: gpui::Rgba,
    pub sidebar_background: gpui::Rgba,
    pub code_background: gpui::Rgba,
    pub success: gpui::Rgba,
    pub warning: gpui::Rgba,
    pub error: gpui::Rgba,
    // JSON 语法高亮颜色
    pub json_key: gpui::Rgba,
    pub json_string: gpui::Rgba,
    pub json_number: gpui::Rgba,
    pub json_boolean: gpui::Rgba,
    pub json_null: gpui::Rgba,
    pub json_bracket: gpui::Rgba,
}

/// 暖色主题
/// 暖色主题 — 温暖护眼
/// 暖色主题 — 羊皮纸质感，典雅舒适
/// 暖色主题 — 羊皮纸质感，典雅舒适
/// 暖色主题 — 羊皮纸质感，典雅舒适
/// 暖色主题
pub fn sepia_theme() -> Theme {
    Theme {
        name: "sepia".to_string(),
        background: rgb(0x2d2420),
        foreground: rgb(0xe8d5c4),
        muted_foreground: rgb(0xa09080),
        accent: rgb(0xd4854a),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0x3d3228),
        border: rgb(0x5a4a38),
        muted_background: rgb(0x3d3228),
        sidebar_background: rgb(0x2d2420),
        code_background: rgb(0x3d3228),
        success: rgb(0x8b9a6b),
        warning: rgb(0xcc9a44),
        error: rgb(0xc4554d),
        json_key: rgb(0xd4a560),
        json_string: rgb(0xc49b6c),
        json_number: rgb(0x8b9a6b),
        json_boolean: rgb(0x9bb8cf),
        json_null: rgb(0x908070),
        json_bracket: rgb(0xe8d5c4),
    }
}

/// 海洋蓝主题
pub fn ocean_theme() -> Theme {
    Theme {
        name: "ocean".to_string(),
        background: rgb(0x0b1929),
        foreground: rgb(0xd4e4f7),
        muted_foreground: rgb(0x7b98b3),
        accent: rgb(0x00b4d8),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0x12283d),
        border: rgb(0x1e3a52),
        muted_background: rgb(0x12283d),
        sidebar_background: rgb(0x0b1929),
        code_background: rgb(0x12283d),
        success: rgb(0x2dd4bf),
        warning: rgb(0xfbbf24),
        error: rgb(0xf87171),
        json_key: rgb(0x67e8f9),
        json_string: rgb(0xfda4af),
        json_number: rgb(0x86efac),
        json_boolean: rgb(0x93c5fd),
        json_null: rgb(0x94a3b8),
        json_bracket: rgb(0xfde68a),
    }
}

/// 日暮橙主题
pub fn sunset_theme() -> Theme {
    Theme {
        name: "sunset".to_string(),
        background: rgb(0x1a1025),
        foreground: rgb(0xf0d9b5),
        muted_foreground: rgb(0x9e8a7a),
        accent: rgb(0xf97316),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0x2a1835),
        border: rgb(0x4a2a45),
        muted_background: rgb(0x2a1835),
        sidebar_background: rgb(0x1a1025),
        code_background: rgb(0x2a1835),
        success: rgb(0x84cc16),
        warning: rgb(0xf59e0b),
        error: rgb(0xef4444),
        json_key: rgb(0xfb923c),
        json_string: rgb(0xfbbf24),
        json_number: rgb(0xa3e635),
        json_boolean: rgb(0x818cf8),
        json_null: rgb(0x9e8a7a),
        json_bracket: rgb(0xf0d9b5),
    }
}

/// 森林绿主题
pub fn forest_theme() -> Theme {
    Theme {
        name: "forest".to_string(),
        background: rgb(0x0d1f17),
        foreground: rgb(0xc8d6c0),
        muted_foreground: rgb(0x6b8a6e),
        accent: rgb(0x4ade80),
        accent_foreground: rgb(0x0d1f17),
        input_background: rgb(0x162e21),
        border: rgb(0x234a31),
        muted_background: rgb(0x162e21),
        sidebar_background: rgb(0x0d1f17),
        code_background: rgb(0x162e21),
        success: rgb(0x86efac),
        warning: rgb(0xfacc15),
        error: rgb(0xf87171),
        json_key: rgb(0x6ee7b7),
        json_string: rgb(0xfca5a5),
        json_number: rgb(0xa3e635),
        json_boolean: rgb(0x67e8f9),
        json_null: rgb(0x6b8a6e),
        json_bracket: rgb(0xc8d6c0),
    }
}

/// Monokai 经典主题
pub fn monokai_theme() -> Theme {
    Theme {
        name: "monokai".to_string(),
        background: rgb(0x272822),
        foreground: rgb(0xf8f8f2),
        muted_foreground: rgb(0x75715e),
        accent: rgb(0xa6e22e),
        accent_foreground: rgb(0x272822),
        input_background: rgb(0x3e3d32),
        border: rgb(0x49483e),
        muted_background: rgb(0x3e3d32),
        sidebar_background: rgb(0x272822),
        code_background: rgb(0x3e3d32),
        success: rgb(0xa6e22e),
        warning: rgb(0xe6db74),
        error: rgb(0xf92672),
        json_key: rgb(0x66d9ef),
        json_string: rgb(0xe6db74),
        json_number: rgb(0xae81ff),
        json_boolean: rgb(0xae81ff),
        json_null: rgb(0x75715e),
        json_bracket: rgb(0xf8f8f2),
    }
}

/// Nord 冷淡主题
pub fn nord_theme() -> Theme {
    Theme {
        name: "nord".to_string(),
        background: rgb(0x2e3440),
        foreground: rgb(0xeceff4),
        muted_foreground: rgb(0x81a1c1),
        accent: rgb(0x88c0d0),
        accent_foreground: rgb(0x2e3440),
        input_background: rgb(0x3b4252),
        border: rgb(0x4c566a),
        muted_background: rgb(0x3b4252),
        sidebar_background: rgb(0x2e3440),
        code_background: rgb(0x3b4252),
        success: rgb(0xa3be8c),
        warning: rgb(0xebcb8b),
        error: rgb(0xbf616a),
        json_key: rgb(0x81a1c1),
        json_string: rgb(0xa3be8c),
        json_number: rgb(0xb48ead),
        json_boolean: rgb(0x81a1c1),
        json_null: rgb(0x4c566a),
        json_bracket: rgb(0xeceff4),
    }
}

/// Dracula — 紫调高对比，夜间长时间编码友好
pub fn dracula_theme() -> Theme {
    Theme {
        name: "dracula".to_string(),
        background: rgb(0x282a36),
        foreground: rgb(0xf8f8f2),
        muted_foreground: rgb(0x6272a4),
        accent: rgb(0xbd93f9),
        accent_foreground: rgb(0x282a36),
        input_background: rgb(0x44475a),
        border: rgb(0x6272a4),
        muted_background: rgb(0x44475a),
        sidebar_background: rgb(0x21222c),
        code_background: rgb(0x282a36),
        success: rgb(0x50fa7b),
        warning: rgb(0xf1fa8c),
        error: rgb(0xff5555),
        json_key: rgb(0x8be9fd),
        json_string: rgb(0xf1fa8c),
        json_number: rgb(0xbd93f9),
        json_boolean: rgb(0xff79c6),
        json_null: rgb(0x6272a4),
        json_bracket: rgb(0xf8f8f2),
    }
}

/// Tokyo Night — 深蓝夜色的霓虹感，冷色但不过分刺眼
pub fn tokyonight_theme() -> Theme {
    Theme {
        name: "tokyonight".to_string(),
        background: rgb(0x1a1b26),
        foreground: rgb(0xc0caf5),
        // 次级色只比原来提了两档亮度：原值 #565f89 压在 #1a1b26 上对比度只有 2.76，
        // 低于 WCAG 对非文本图标的 3.0；#5f6b96 是同色相邻近的蓝灰，提亮后 3.28，
        // 既达标又与 foreground(#c0caf5, 3.23) 拉开，保持"次级"而非"正文"观感
        muted_foreground: rgb(0x5f6b96),
        accent: rgb(0x7aa2f7),
        accent_foreground: rgb(0x1a1b26),
        input_background: rgb(0x24283b),
        border: rgb(0x3b4261),
        muted_background: rgb(0x24283b),
        sidebar_background: rgb(0x16161e),
        code_background: rgb(0x1f2335),
        success: rgb(0x9ece6a),
        warning: rgb(0xe0af68),
        error: rgb(0xf7768e),
        json_key: rgb(0x7aa2f7),
        json_string: rgb(0x9ece6a),
        json_number: rgb(0xff9e64),
        json_boolean: rgb(0xbb9af7),
        json_null: rgb(0x565f89),
        json_bracket: rgb(0xc0caf5),
    }
}

/// Gruvbox Dark — 复古暖色，低蓝光，长时间看不累
pub fn gruvbox_theme() -> Theme {
    Theme {
        name: "gruvbox".to_string(),
        background: rgb(0x282828),
        foreground: rgb(0xebdbb2),
        muted_foreground: rgb(0x928374),
        accent: rgb(0xfabd2f),
        accent_foreground: rgb(0x282828),
        input_background: rgb(0x3c3836),
        border: rgb(0x504945),
        muted_background: rgb(0x3c3836),
        sidebar_background: rgb(0x1d2021),
        code_background: rgb(0x32302f),
        success: rgb(0xb8bb26),
        warning: rgb(0xfabd2f),
        error: rgb(0xfb4934),
        json_key: rgb(0x83a598),
        json_string: rgb(0xb8bb26),
        json_number: rgb(0xd3869b),
        json_boolean: rgb(0xfabd2f),
        json_null: rgb(0x928374),
        json_bracket: rgb(0xebdbb2),
    }
}

/// Catppuccin Latte — 浅色，柔和低对比的奶咖灰底，白天不刺眼
pub fn latte_theme() -> Theme {
    Theme {
        name: "latte".to_string(),
        background: rgb(0xeff1f5),
        foreground: rgb(0x4c4f69),
        // 与 tokyonight 同理：原值 #8c8fa1 压在 #eff1f5 上只有 2.83，
        // #878a9c 是同色相提亮一档（3.02），刚好越过 3.0 又不贴近 foreground
        muted_foreground: rgb(0x878a9c),
        accent: rgb(0x7287fd),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0xe6e9ef),
        border: rgb(0xccd0da),
        muted_background: rgb(0xe6e9ef),
        sidebar_background: rgb(0xe6e9ef),
        code_background: rgb(0xf7f8fa),
        success: rgb(0x40a02b),
        warning: rgb(0xdf8e1d),
        error: rgb(0xd20f39),
        json_key: rgb(0x1e66f5),
        json_string: rgb(0x40a02b),
        json_number: rgb(0xfe640b),
        json_boolean: rgb(0x8839ef),
        json_null: rgb(0x8c8fa1),
        json_bracket: rgb(0x4c4f69),
    }
}

impl Theme {
    /// 从字符串获取主题
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => light_theme(),
            "sepia" => sepia_theme(),
            "ocean" => ocean_theme(),
            "sunset" => sunset_theme(),
            "forest" => forest_theme(),
            "monokai" => monokai_theme(),
            "nord" => nord_theme(),
            "dracula" => dracula_theme(),
            "tokyonight" => tokyonight_theme(),
            "gruvbox" => gruvbox_theme(),
            "latte" => latte_theme(),
            _ => dark_theme(),
        }
    }

    /// 全部主题名（顺序即 Ctrl+T 的循环顺序、设置面板的网格顺序）
    pub const NAMES: &'static [&'static str] = &[
        "dark", "light", "sepia", "ocean", "sunset", "forest", "monokai", "nord", "dracula",
        "tokyonight", "gruvbox", "latte",
    ];

    /// 该主题是否属于浅色底色。
    ///
    /// 用于决定 gpui_component 组件库用 Light 还是 Dark 模式：
    /// 底色浅而组件按 Dark 模式渲染会出现对比度问题。
    pub fn is_light(name: &str) -> bool {
        matches!(name, "light" | "latte")
    }
}
