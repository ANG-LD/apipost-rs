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
    }
}

/// 扩展Theme结构以包含更多颜色
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
}

impl Theme {
    /// 从字符串获取主题
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => light_theme(),
            "dark" | _ => dark_theme(),
        }
    }
}
