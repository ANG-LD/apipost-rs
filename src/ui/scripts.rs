//! 脚本模块
//!
//! Pre-request Script 和 Test Script 管理

use gpui_component::input::{Input, InputState};
use gpui::*;

/// 脚本状态
#[derive(Clone)]
pub struct ScriptState {
    pub pre_request_script: Entity<InputState>,
    pub test_script: Entity<InputState>,
}

impl ScriptState {
    /// 创建新的脚本状态
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        let pre_request_script = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("")
                .placeholder("// Run code before request is sent")
        });
        let test_script = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("")
                .placeholder("// Run tests after response is received")
        });
        Self {
            pre_request_script,
            test_script,
        }
    }

    /// 获取前置脚本内容
    pub fn get_pre_request_script(&self, cx: &Context<crate::ui::MainView>) -> String {
        self.pre_request_script.read(cx).value().to_string()
    }

    /// 获取测试脚本内容
    pub fn get_test_script(&self, cx: &Context<crate::ui::MainView>) -> String {
        self.test_script.read(cx).value().to_string()
    }
}
