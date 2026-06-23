//! 主题模块
//!
//! 定义应用的主题色彩方案

use gpui::*;

/// 浅色主题
pub fn light_theme() -> Theme {
    Theme {
        name: "light".to_string(),
        background: rgb(0xffffff),
        foreground: rgb(0x1f2937),
        muted_foreground: rgb(0x6b7280),
        accent: rgb(0x3b82f6),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0xf9fafb),
        border: rgb(0xe5e7eb),
        muted_background: rgb(0xf3f4f6),
        sidebar_background: rgb(0xf9fafb),
        code_background: rgb(0xf3f4f6),
        success: rgb(0x22c55e),
        warning: rgb(0xf59e0b),
        error: rgb(0xef4444),
        // JSON 语法高亮颜色（浅色主题）
        json_key: rgb(0x0078d4),      // 蓝色 - JSON键
        json_string: rgb(0xa31515),   // 红色 - 字符串值
        json_number: rgb(0x098658),   // 绿色 - 数字
        json_boolean: rgb(0x0000ff),  // 蓝色 - 布尔值
        json_null: rgb(0x808080),     // 灰色 - null
        json_bracket: rgb(0x000000),  // 黑色 - 括号
    }
}

/// 深色主题
pub fn dark_theme() -> Theme {
    Theme {
        name: "dark".to_string(),
        background: rgb(0x0f172a),
        foreground: rgb(0xf1f5f9),
        muted_foreground: rgb(0x94a3b8),
        accent: rgb(0x3b82f6),
        accent_foreground: rgb(0xffffff),
        input_background: rgb(0x1e293b),
        border: rgb(0x334155),
        muted_background: rgb(0x1e293b),
        sidebar_background: rgb(0x0f172a),
        code_background: rgb(0x1e293b),
        success: rgb(0x22c55e),
        warning: rgb(0xf59e0b),
        error: rgb(0xef4444),
        // JSON 语法高亮颜色（深色主题）- 使用更亮的颜色确保可见性
        json_key: rgb(0x4fc1ff),      // 亮蓝色 - JSON键
        json_string: rgb(0xff8c69),   // 亮橙色 - 字符串值
        json_number: rgb(0x98d977),   // 亮绿色 - 数字
        json_boolean: rgb(0x79b8ff),  // 亮蓝色 - 布尔值
        json_null: rgb(0xe0e0e0),     // 亮灰色 - null
        json_bracket: rgb(0xffdd59),  // 亮黄色 - 括号
    }
}

/// 扩展Theme结构以包含更多颜色
#[derive(Clone)]
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
            _ => dark_theme(),
        }
    }
}
