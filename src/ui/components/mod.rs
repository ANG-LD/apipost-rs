//! UI组件模块
//!
//! 包含可复用的UI组件
//! 注意：此模块需要使用gpui-component进行重写

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

// TODO: 使用gpui-component重写以下UI组件
// - method_badge
// - status_badge
// - text_input
// - button
// - secondary_button
// - icon_button
