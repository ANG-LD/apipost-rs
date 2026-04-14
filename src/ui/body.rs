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

/// Form Data 参数类型
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FormDataParamType {
    Text,
    Boolean,
    Number,
    File,
    Array,
}

impl Default for FormDataParamType {
    fn default() -> Self {
        FormDataParamType::Text
    }
}

impl FormDataParamType {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => FormDataParamType::Text,
            1 => FormDataParamType::Boolean,
            2 => FormDataParamType::Number,
            3 => FormDataParamType::File,
            4 => FormDataParamType::Array,
            _ => FormDataParamType::Text,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            FormDataParamType::Text => 0,
            FormDataParamType::Boolean => 1,
            FormDataParamType::Number => 2,
            FormDataParamType::File => 3,
            FormDataParamType::Array => 4,
        }
    }

    pub fn all() -> Vec<gpui::SharedString> {
        vec![
            "Text".into(),
            "Boolean".into(),
            "Number".into(),
            "File".into(),
            "Array".into(),
        ]
    }

    pub fn create_select(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Entity<SelectState<Vec<gpui::SharedString>>> {
        let types = Self::all();
        cx.new(|cx| {
            SelectState::new(types, Some(IndexPath::default()), window, cx)
        })
    }
}

/// Form Data 条目类型
#[derive(Clone)]
pub enum FormDataValue {
    /// 文本值
    Text(Entity<InputState>),
    /// 文件上传 (文件路径存储, 用于显示已选文件)
    File(Entity<InputState>, String),
}

impl FormDataValue {
    /// 获取输入实体
    pub fn get_input_entity(&self) -> Entity<InputState> {
        match self {
            FormDataValue::Text(e) => e.clone(),
            FormDataValue::File(e, _) => e.clone(),
        }
    }
}

/// Form Data 条目
#[derive(Clone)]
pub struct FormDataEntry {
    pub key: Entity<InputState>,
    pub value: FormDataValue,
    pub enabled: bool,
    pub param_type: FormDataParamType,
    pub type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
}

/// Body 状态
#[derive(Clone)]
pub struct BodyState {
    pub body_type: BodyType,
    pub raw_format: RawFormat,
    pub raw_content: Entity<InputState>,
    /// Form-data 条目列表
    pub form_data: Vec<FormDataEntry>,
    /// URL-encoded 条目列表
    pub urlencoded_data: Vec<FormDataEntry>,
}

impl BodyState {
    /// 创建 Body 状态
    pub fn new(raw_content: Entity<InputState>) -> Self {
        Self {
            body_type: BodyType::None,
            raw_format: RawFormat::Json,
            raw_content,
            form_data: Vec::new(),
            urlencoded_data: Vec::new(),
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

    /// 获取 form-data 文本字段 (key-value pairs)
    pub fn get_form_data_text_fields(&self, cx: &Context<crate::ui::MainView>) -> Vec<(String, String)> {
        let mut fields = Vec::new();
        for entry in &self.form_data {
            if entry.enabled && entry.param_type != FormDataParamType::File {
                let key = entry.key.read(cx).value().to_string();
                let value = entry.value.get_input_entity().read(cx).value().to_string();
                if !key.is_empty() {
                    fields.push((key, value));
                }
            }
        }
        fields
    }

    /// 获取 form-data 文件字段
    pub fn get_form_data_file_fields(&self, cx: &Context<crate::ui::MainView>) -> Vec<(String, String, String)> {
        let mut fields = Vec::new();
        for entry in &self.form_data {
            if entry.enabled && entry.param_type == FormDataParamType::File {
                let key = entry.key.read(cx).value().to_string();
                if let FormDataValue::File(_, ref file_path) = entry.value {
                    if !key.is_empty() && !file_path.is_empty() {
                        // 从文件扩展名推断 content-type
                        let content_type = guess_content_type(file_path);
                        fields.push((key, file_path.clone(), content_type));
                    }
                }
            }
        }
        fields
    }

    /// 添加 form-data 条目
    pub fn add_form_data_entry(&mut self, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let type_select = FormDataParamType::create_select(window, cx);
        self.form_data.push(FormDataEntry {
            key,
            value: FormDataValue::Text(value),
            enabled: true,
            param_type: FormDataParamType::Text,
            type_select,
        });
        cx.notify();
    }

    /// 添加 url-encoded 条目
    pub fn add_urlencoded_entry(&mut self, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        let key = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let type_select = FormDataParamType::create_select(window, cx);
        self.urlencoded_data.push(FormDataEntry {
            key,
            value: FormDataValue::Text(value),
            enabled: true,
            param_type: FormDataParamType::Text,
            type_select,
        });
        cx.notify();
    }

    /// 设置 form-data 条目类型
    pub fn set_form_data_param_type(&mut self, index: usize, param_type: FormDataParamType, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        if index < self.form_data.len() {
            let entry = &mut self.form_data[index];
            entry.param_type = param_type;
            entry.value = if param_type == FormDataParamType::File {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(""));
                FormDataValue::File(input, String::new())
            } else {
                let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
                FormDataValue::Text(value)
            };
        }
    }

    /// 更新 form-data File 条目的文件路径
    pub fn update_form_data_file_path(&mut self, index: usize, file_path: &str, _window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        if index < self.form_data.len() {
            let entry = &mut self.form_data[index];
            if let FormDataValue::File(ref input_entity, ref mut stored_path) = entry.value {
                *stored_path = file_path.to_string();
                // 同时更新输入框显示
                let path_owned = file_path.to_string();
                input_entity.update(cx, move |state, _cx| {
                    state.set_value(&path_owned, _window, _cx);
                });
            }
        }
    }

    /// 获取 form-data File 条目的文件路径
    pub fn get_form_data_file_path(&self, index: usize) -> Option<String> {
        if index < self.form_data.len() {
            if let FormDataValue::File(_, ref stored_path) = self.form_data[index].value {
                if !stored_path.is_empty() {
                    return Some(stored_path.clone());
                }
            }
        }
        None
    }

    /// 设置 url-encoded 条目类型
    pub fn set_urlencoded_param_type(&mut self, index: usize, param_type: FormDataParamType, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        if index < self.urlencoded_data.len() {
            let entry = &mut self.urlencoded_data[index];
            entry.param_type = param_type;
            entry.value = if param_type == FormDataParamType::File {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(""));
                FormDataValue::File(input, String::new())
            } else {
                let value = cx.new(|cx| InputState::new(window, cx).default_value(""));
                FormDataValue::Text(value)
            };
        }
    }

    /// 删除 form-data 条目
    pub fn remove_form_data_entry(&mut self, index: usize) {
        if index < self.form_data.len() {
            self.form_data.remove(index);
        }
    }

    /// 删除 url-encoded 条目
    pub fn remove_urlencoded_entry(&mut self, index: usize) {
        if index < self.urlencoded_data.len() {
            self.urlencoded_data.remove(index);
        }
    }

    /// 切换 form-data 条目启用状态
    pub fn toggle_form_data_entry(&mut self, index: usize) {
        if index < self.form_data.len() {
            self.form_data[index].enabled = !self.form_data[index].enabled;
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

/// 根据文件扩展名推断内容类型
fn guess_content_type(file_path: &str) -> String {
    let ext = std::path::Path::new(file_path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "gzip" => "application/gzip",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "ts" => "application/typescript",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    }
    .to_string()
}
