//! 请求体模块
//!
//! 支持多种 Body 类型：none, form-data, x-www-form-urlencoded, raw, binary

use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::{IndexPath, Sizable, StyledExt};
use gpui::*;

/// Body 类型枚举
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BodyType {
    None,
    FormData,
    UrlEncoded,
    Raw,
    Binary,
}

impl Default for BodyType {
    fn default() -> Self {
        BodyType::None
    }
}

impl BodyType {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => BodyType::None,
            1 => BodyType::FormData,
            2 => BodyType::UrlEncoded,
            3 => BodyType::Raw,
            4 => BodyType::Binary,
            _ => BodyType::None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            BodyType::None => 0,
            BodyType::FormData => 1,
            BodyType::UrlEncoded => 2,
            BodyType::Raw => 3,
            BodyType::Binary => 4,
        }
    }

    pub fn all() -> Vec<gpui::SharedString> {
        vec![
            "none".into(),
            "form-data".into(),
            "x-www-form-urlencoded".into(),
            "raw".into(),
            "binary".into(),
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            BodyType::None => "none",
            BodyType::FormData => "form-data",
            BodyType::UrlEncoded => "x-www-form-urlencoded",
            BodyType::Raw => "raw",
            BodyType::Binary => "binary",
        }
    }
}

/// Raw 格式类型
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RawFormat {
    Json,
    Xml,
    Text,
    Html,
    JavaScript,
}

impl Default for RawFormat {
    fn default() -> Self {
        RawFormat::Json
    }
}

impl RawFormat {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => RawFormat::Json,
            1 => RawFormat::Xml,
            2 => RawFormat::Text,
            3 => RawFormat::Html,
            4 => RawFormat::JavaScript,
            _ => RawFormat::Json,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            RawFormat::Json => 0,
            RawFormat::Xml => 1,
            RawFormat::Text => 2,
            RawFormat::Html => 3,
            RawFormat::JavaScript => 4,
        }
    }

    pub fn all() -> Vec<gpui::SharedString> {
        vec![
            "JSON".into(),
            "XML".into(),
            "Text".into(),
            "HTML".into(),
            "JavaScript".into(),
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            RawFormat::Json => "JSON",
            RawFormat::Xml => "XML",
            RawFormat::Text => "Text",
            RawFormat::Html => "HTML",
            RawFormat::JavaScript => "JavaScript",
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            RawFormat::Json => "application/json",
            RawFormat::Xml => "application/xml",
            RawFormat::Text => "text/plain",
            RawFormat::Html => "text/html",
            RawFormat::JavaScript => "application/javascript",
        }
    }
}

/// Form Data 条目
#[derive(Clone)]
pub struct FormDataEntry {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub enabled: bool,
}

/// Body 状态
#[derive(Clone)]
pub struct BodyState {
    pub body_type: BodyType,
    pub raw_format: RawFormat,
    pub raw_content: Entity<InputState>,
}

impl BodyState {
    /// 创建 Body 状态
    pub fn new(raw_content: Entity<InputState>) -> Self {
        Self {
            body_type: BodyType::None,
            raw_format: RawFormat::Json,
            raw_content,
        }
    }

    /// 获取 Content-Type
    pub fn content_type(&self) -> Option<String> {
        match self.body_type {
            BodyType::None => None,
            BodyType::FormData => Some("multipart/form-data".to_string()),
            BodyType::UrlEncoded => Some("application/x-www-form-urlencoded".to_string()),
            BodyType::Raw => Some(format!("{}", self.raw_format.content_type())),
            BodyType::Binary => None,
        }
    }

    /// 获取请求体内容
    pub fn to_body(&self, cx: &Context<crate::ui::MainView>) -> Option<String> {
        match self.body_type {
            BodyType::None => None,
            BodyType::Raw => {
                let content = self.raw_content.read(cx).value().to_string();
                if content.is_empty() { None } else { Some(content) }
            }
            BodyType::FormData | BodyType::UrlEncoded | BodyType::Binary => {
                // 简化实现，实际应该用 form-data 编码
                let content = self.raw_content.read(cx).value().to_string();
                if content.is_empty() { None } else { Some(content) }
            }
        }
    }

    /// 创建 Body 类型选择器
    pub fn create_body_type_select(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Entity<SelectState<Vec<gpui::SharedString>>> {
        let body_types = BodyType::all();
        cx.new(|cx| {
            SelectState::new(body_types, Some(IndexPath::default()), window, cx)
        })
    }

    /// 创建 Raw 格式选择器
    pub fn create_raw_format_select(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Entity<SelectState<Vec<gpui::SharedString>>> {
        let raw_formats = RawFormat::all();
        cx.new(|cx| {
            SelectState::new(raw_formats, Some(IndexPath::default()), window, cx)
        })
    }
}
