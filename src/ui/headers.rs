//! Headers 模块
//!
//! 请求头表格形式管理

use gpui_component::input::{Input, InputState};
use gpui::*;

/// Header 条目
#[derive(Clone)]
pub struct HeaderEntry {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub enabled: bool,
}

impl HeaderEntry {
    /// 创建新的 Header 条目
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
        Self { key, value, enabled: true }
    }

    /// 获取键值对
    pub fn get_key_value(&self, cx: &Context<crate::ui::MainView>) -> (String, String) {
        let key = self.key.read(cx).value().to_string();
        let value = self.value.read(cx).value().to_string();
        (key, value)
    }
}

/// Headers 状态管理
pub struct HeadersState {
    pub headers: Vec<HeaderEntry>,
}

impl Default for HeadersState {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
        }
    }
}

impl HeadersState {
    /// 添加新的 Header 条目
    pub fn add_header(&mut self, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        self.headers.push(HeaderEntry::new(window, cx));
    }

    /// 删除指定索引的 Header 条目
    pub fn remove_header(&mut self, index: usize) {
        if index < self.headers.len() {
            self.headers.remove(index);
        }
    }

    /// 切换 Header 启用状态
    pub fn toggle_header(&mut self, index: usize) {
        if index < self.headers.len() {
            self.headers[index].enabled = !self.headers[index].enabled;
        }
    }

    /// 获取所有启用的 Headers
    pub fn get_enabled_headers(&self, cx: &Context<crate::ui::MainView>) -> Vec<(String, String)> {
        self.headers
            .iter()
            .filter(|h| h.enabled)
            .filter_map(|h| {
                let (key, value) = h.get_key_value(cx);
                if key.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect()
    }
}
