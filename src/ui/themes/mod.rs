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

impl Theme {
    /// 从字符串获取主题
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => light_theme(),
            "sepia" => sepia_theme(),
            _ => dark_theme(),
        }
    }
}
