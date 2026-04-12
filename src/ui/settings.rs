//! 设置模块
//!
//! 请求设置管理：超时、重试、跟随重定向、SSL验证等

use gpui_component::input::{Input, InputState};
use gpui_component::{Sizable, StyledExt};
use gpui::*;

/// 请求设置
#[derive(Clone)]
pub struct RequestSettings {
    pub timeout_secs: u64,
    pub retry_count: u32,
    pub follow_redirects: bool,
    pub verify_ssl: bool,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            retry_count: 0,
            follow_redirects: true,
            verify_ssl: true,
        }
    }
}

impl RequestSettings {
    /// 创建设置
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        Self::default()
    }
}

/// 设置相关输入状态
pub struct SettingsInputs {
    pub timeout_input: Entity<InputState>,
    pub retry_input: Entity<InputState>,
}

impl SettingsInputs {
    /// 创建新的设置输入状态
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        let timeout_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("30")
        });
        let retry_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("0")
        });
        Self {
            timeout_input,
            retry_input,
        }
    }

    /// 获取超时秒数
    pub fn get_timeout_secs(&self, cx: &Context<crate::ui::MainView>) -> u64 {
        let value = self.timeout_input.read(cx).value().to_string();
        value.parse().unwrap_or(30)
    }

    /// 获取重试次数
    pub fn get_retry_count(&self, cx: &Context<crate::ui::MainView>) -> u32 {
        let value = self.retry_input.read(cx).value().to_string();
        value.parse().unwrap_or(0)
    }
}
