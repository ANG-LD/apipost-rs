//! 请求体模块
//!
//! 支持多种 Body 类型：none, form-data, x-www-form-urlencoded, raw, binary

use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::{IndexPath, Sizable, StyledExt};
use gpui::*;
use regex;

/// Body 类型枚举
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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

    /// 检测响应格式
    pub fn detect(content_type: Option<&str>, body: &str) -> RawFormat {
        if let Some(ct) = content_type {
            let ct_lower = ct.to_lowercase();
            if ct_lower.contains("json") {
                return RawFormat::Json;
            } else if ct_lower.contains("xml") {
                return RawFormat::Xml;
            } else if ct_lower.contains("html") {
                return RawFormat::Html;
            } else if ct_lower.contains("javascript") {
                return RawFormat::JavaScript;
            } else {
                return RawFormat::Text;
            }
        }
        // 尝试基于内容推断
        let body_lower = body.trim_start();
        if body_lower.starts_with('{') || body_lower.starts_with('[') {
            return RawFormat::Json;
        } else if body_lower.starts_with('<') {
            if body_lower.contains("<!DOCTYPE html") || body_lower.contains("<html") {
                return RawFormat::Html;
            }
            return RawFormat::Xml;
        }
        RawFormat::Text
    }

    /// 根据格式格式化内容
    pub fn format_body(&self, body: &str) -> String {
        match self {
            RawFormat::Json => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string())
                } else {
                    body.to_string()
                }
            }
            RawFormat::Xml => body.to_string(),
            RawFormat::Text => body.to_string(),
            RawFormat::Html => body.to_string(),
            RawFormat::JavaScript => body.to_string(),
        }
    }
}

/// Form Data 参数类型
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
    /// XML 格式的 raw_content
    pub raw_content_xml: Entity<InputState>,
    /// Text 格式的 raw_content
    pub raw_content_text: Entity<InputState>,
    /// HTML 格式的 raw_content
    pub raw_content_html: Entity<InputState>,
    /// Form-data 条目列表
    pub form_data: Vec<FormDataEntry>,
    /// URL-encoded 条目列表
    pub urlencoded_data: Vec<FormDataEntry>,
    /// Raw 编辑器高度
    pub raw_editor_height: f32,
    /// 是否正在拖动调整大小
    pub is_resizing: bool,
    /// 拖动开始时的鼠标 Y 坐标
    pub resize_start_y: f32,
    /// 拖动开始时的高度
    pub resize_start_height: f32,
    /// 上次拖动更新时间（用于节流）
    last_drag_update: Option<std::time::Instant>,
    /// JSON 格式化错误信息
    pub json_error: Option<String>,
    /// 软换行开关
    pub soft_wrap: bool,
}

impl BodyState {
    /// 创建 Body 状态
    pub fn new(
        raw_content: Entity<InputState>,
        raw_content_xml: Entity<InputState>,
        raw_content_text: Entity<InputState>,
        raw_content_html: Entity<InputState>,
    ) -> Self {
        Self {
            body_type: BodyType::None,
            raw_format: RawFormat::Json,
            raw_content,
            raw_content_xml,
            raw_content_text,
            raw_content_html,
            form_data: Vec::new(),
            urlencoded_data: Vec::new(),
            raw_editor_height: 300.0,
            is_resizing: false,
            resize_start_y: 0.0,
            resize_start_height: 300.0,
            last_drag_update: None,
            json_error: None,
            soft_wrap: false,
        }
    }

    /// 获取 Content-Type
    pub fn content_type(&self) -> Option<String> {
        match self.body_type {
            BodyType::None => None,
            BodyType::FormData => {
                // 只有当 form_data 有实际条目时才返回 multipart content-type
                if self.form_data.is_empty() {
                    None
                } else {
                    Some("multipart/form-data".to_string())
                }
            }
            BodyType::UrlEncoded => {
                // 只有当 urlencoded_data 有实际条目时才返回 urlencoded content-type
                if self.urlencoded_data.is_empty() {
                    None
                } else {
                    Some("application/x-www-form-urlencoded".to_string())
                }
            }
            BodyType::Raw => Some(format!("{}", self.raw_format.content_type())),
            BodyType::Binary => None,
        }
    }

    /// 获取请求体内容
    pub fn to_body(&self, cx: &Context<crate::ui::MainView>) -> Option<String> {
        match self.body_type {
            BodyType::None => None,
            BodyType::Raw => {
                let content = match self.raw_format {
                    RawFormat::Json => self.raw_content.read(cx).value().to_string(),
                    RawFormat::Xml => self.raw_content_xml.read(cx).value().to_string(),
                    RawFormat::Text => self.raw_content_text.read(cx).value().to_string(),
                    RawFormat::Html => self.raw_content_html.read(cx).value().to_string(),
                    RawFormat::JavaScript => self.raw_content.read(cx).value().to_string(),
                };
                if content.is_empty() { None } else { Some(content) }
            }
            BodyType::FormData | BodyType::UrlEncoded | BodyType::Binary => {
                // FormData/UrlEncoded/Binary 的 body 由 text_fields/file_fields 处理
                // to_body() 返回 None（body 内容已经在 http client 的 multipart 中处理）
                None
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

    /// 设置 Raw 编辑器高度
    pub fn set_raw_editor_height(&mut self, height: f32) {
        self.raw_editor_height = height.max(100.0).min(800.0);
    }

    /// 切换软换行
    pub fn toggle_soft_wrap(&mut self) {
        self.soft_wrap = !self.soft_wrap;
    }

    /// 格式化 Raw 类型的 JSON 内容
    pub fn format_json(&mut self, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        if self.body_type != BodyType::Raw || self.raw_format != RawFormat::Json {
            return;
        }

        let text = self.raw_content.read(cx).value().to_string();
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => {
                let formatted = serde_json::to_string_pretty(&value).unwrap_or(text.clone());
                let formatted_owned = formatted;
                self.raw_content.update(cx, move |this, _cx| {
                    this.set_value(&formatted_owned, window, _cx);
                });
                self.json_error = None;
            }
            Err(e) => {
                self.json_error = Some(e.to_string());
            }
        }
    }

    /// 开始拖动调整大小
    pub fn start_resize(&mut self, mouse_y: f32) {
        self.is_resizing = true;
        self.resize_start_y = mouse_y;
        self.resize_start_height = self.raw_editor_height;
    }

    /// 更新拖动（根据鼠标Y坐标计算新高度，带节流）
    /// 返回是否需要重绘
    pub fn update_resize(&mut self, mouse_y: f32) -> bool {
        if self.is_resizing {
            let now = std::time::Instant::now();
            // 节流：限制更新间隔为 60fps (约 16ms)
            if let Some(last) = self.last_drag_update {
                if now.duration_since(last).as_millis() < 16 {
                    return false;
                }
            }
            self.last_drag_update = Some(now);

            // delta 为正表示鼠标向下移动，编辑器应该变高
            let delta = mouse_y - self.resize_start_y;
            let new_height = self.resize_start_height + delta;

            // 边界检测：只在有效范围内触发重绘
            if new_height >= 100.0 && new_height <= 800.0 {
                self.raw_editor_height = new_height;
                return true;
            }
            // 达到边界时不触发重绘，避免"卡顿"感
            return false;
        }
        false
    }

    /// 结束拖动调整大小
    pub fn end_resize(&mut self) {
        self.is_resizing = false;
        self.last_drag_update = None;
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
            // 同步 type_select 下拉框的选中项
            entry.type_select.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(param_type.to_index())), window, cx);
            });
            entry.value = if param_type == FormDataParamType::File {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(""));
                FormDataValue::File(input, String::new())
            } else if param_type == FormDataParamType::Boolean {
                let value = cx.new(|cx| InputState::new(window, cx).default_value("true"));
                FormDataValue::Text(value)
            } else if param_type == FormDataParamType::Number {
                let num_pattern = regex::Regex::new(r"^-?[0-9]*\.?[0-9]*$").unwrap();
                let value = cx.new(|cx| InputState::new(window, cx).default_value("").pattern(num_pattern));
                FormDataValue::Text(value)
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
            } else if param_type == FormDataParamType::Boolean {
                let value = cx.new(|cx| InputState::new(window, cx).default_value("true"));
                FormDataValue::Text(value)
            } else if param_type == FormDataParamType::Number {
                let num_pattern = regex::Regex::new(r"^-?[0-9]*\.?[0-9]*$").unwrap();
                let value = cx.new(|cx| InputState::new(window, cx).default_value("").pattern(num_pattern));
                FormDataValue::Text(value)
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

/// 可序列化的 Form Data 条目快照
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedFormDataEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub param_type: FormDataParamType,
    pub is_file: bool,
    pub file_path: String,
}

/// JSON 格式化/折叠选项
#[derive(Clone)]
pub struct JsonFormatOptions {
    /// 最大展开深度，0 表示全部展开
    pub max_depth: usize,
    /// 是否启用折叠
    pub enable_fold: bool,
    /// 折叠的缩进宽度
    pub indent_size: usize,
}

impl Default for JsonFormatOptions {
    fn default() -> Self {
        Self {
            max_depth: 3,
            enable_fold: true,
            indent_size: 2,
        }
    }
}

/// 格式化 JSON 字符串
pub fn format_json(json_str: &str, options: &JsonFormatOptions) -> String {
    // 尝试解析 JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if options.enable_fold {
            format_json_value(&value, 0, options.max_depth, options.indent_size)
        } else {
            // 全部展开
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| json_str.to_string())
        }
    } else {
        // 不是有效的 JSON，返回原字符串
        json_str.to_string()
    }
}

/// 递归格式化 JSON 值（带折叠）
fn format_json_value(value: &serde_json::Value, current_depth: usize, max_depth: usize, indent_size: usize) -> String {
    let indent = " ".repeat(current_depth * indent_size);
    let next_indent = " ".repeat((current_depth + 1) * indent_size);

    match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }

            // 如果超过最大深度且深度限制有效（>0），显示折叠提示
            if max_depth > 0 && current_depth >= max_depth {
                let key_count = map.len();
                return format!("{{ {} keys... }}", key_count);
            }

            let mut result = String::from("{\n");
            for (i, (key, val)) in map.iter().enumerate() {
                let comma = if i < map.len() - 1 { "," } else { "" };
                result.push_str(&format!("{next_indent}\"{key}\": {}", format_json_value(val, current_depth + 1, max_depth, indent_size)));
                result.push_str(comma);
                result.push('\n');
            }
            result.push_str(&indent);
            result.push('}');
            result
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }

            // 如果超过最大深度且深度限制有效（>0），显示折叠提示
            if max_depth > 0 && current_depth >= max_depth {
                let len = arr.len();
                return format!("[ {} items... ]", len);
            }

            let mut result = String::from("[\n");
            for (i, val) in arr.iter().enumerate() {
                let comma = if i < arr.len() - 1 { "," } else { "" };
                result.push_str(&format!("{next_indent}{}", format_json_value(val, current_depth + 1, max_depth, indent_size)));
                result.push_str(comma);
                result.push('\n');
            }
            result.push_str(&indent);
            result.push(']');
            result
        }
        serde_json::Value::String(s) => format!("\"{}\"", escape_json_string(s)),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// 转义 JSON 字符串中的特殊字符
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// 紧凑 JSON 字符串（移除空白）
pub fn compact_json(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string())
    } else {
        json_str.to_string()
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

/// JSON 语法高亮类型
#[derive(Clone, Debug)]
pub enum JsonTokenType {
    Key,
    String,
    Number,
    Boolean,
    Null,
    Bracket,
    Comma,
    Colon,
    Plain,
}

/// JSON 语法高亮标记
#[derive(Clone, Debug)]
pub struct JsonHighlightToken {
    pub token_type: JsonTokenType,
    pub text: String,
}

/// 高亮 JSON 字符串（用于渲染）
/// 返回 (文本片段, 颜色) 的列表
pub fn highlight_json(json_str: &str, theme: &crate::ui::themes::Theme) -> Vec<(String, gpui::Rgba)> {
    let mut tokens = Vec::new();
    let mut chars = json_str.chars().peekable();
    let mut pos = 0;

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                // 解析字符串（可能是键或值）
                let start_pos = pos;
                let mut s = String::from('"');
                while let Some(&ch) = chars.peek() {
                    if ch == '"' {
                        s.push(chars.next().unwrap());
                        pos += 1;
                        break;
                    } else if ch == '\\' {
                        s.push(chars.next().unwrap());
                        pos += 1;
                        if let Some(&escaped) = chars.peek() {
                            s.push(escaped);
                            chars.next();
                            pos += 1;
                        }
                    } else {
                        s.push(chars.next().unwrap());
                        pos += 1;
                    }
                }
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::String,
                    text: s,
                });
            }
            '{' | '}' | '[' | ']' => {
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Bracket,
                    text: c.to_string(),
                });
                pos += 1;
            }
            ':' => {
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Colon,
                    text: c.to_string(),
                });
                pos += 1;
            }
            ',' => {
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Comma,
                    text: c.to_string(),
                });
                pos += 1;
            }
            'n' => {
                let rest: String = json_str.chars().skip(pos).take(4).collect();
                if rest == "null" {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Null,
                        text: "null".to_string(),
                    });
                    for _ in 0..4 { chars.next(); pos += 1; }
                } else {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Plain,
                        text: c.to_string(),
                    });
                    pos += 1;
                }
            }
            't' => {
                let rest: String = json_str.chars().skip(pos).take(4).collect();
                if rest == "true" {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Boolean,
                        text: "true".to_string(),
                    });
                    for _ in 0..4 { chars.next(); pos += 1; }
                } else {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Plain,
                        text: c.to_string(),
                    });
                    pos += 1;
                }
            }
            'f' => {
                let rest: String = json_str.chars().skip(pos).take(5).collect();
                if rest == "false" {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Boolean,
                        text: "false".to_string(),
                    });
                    for _ in 0..5 { chars.next(); pos += 1; }
                } else {
                    tokens.push(JsonHighlightToken {
                        token_type: JsonTokenType::Plain,
                        text: c.to_string(),
                    });
                    pos += 1;
                }
            }
            c if c.is_ascii_digit() || c == '-' => {
                let mut num = String::new();
                if c == '-' {
                    num.push(c);
                    pos += 1;
                    if let Some(&next) = chars.peek() {
                        if next.is_ascii_digit() {
                            num.push(chars.next().unwrap());
                            pos += 1;
                        }
                    }
                } else {
                    num.push(c);
                    pos += 1;
                }
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' || next == 'e' || next == 'E' || next == '+' || next == '-' {
                        if (next == '+' || next == '-') && !num.ends_with('e') && !num.ends_with('E') {
                            break;
                        }
                        num.push(chars.next().unwrap());
                        pos += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Number,
                    text: num,
                });
            }
            ' ' | '\t' | '\n' | '\r' => {
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Plain,
                    text: c.to_string(),
                });
                pos += 1;
            }
            _ => {
                tokens.push(JsonHighlightToken {
                    token_type: JsonTokenType::Plain,
                    text: c.to_string(),
                });
                pos += 1;
            }
        }
    }

    // 第二次遍历：将 String 类型修正为 Key 或 String
    let mut result = Vec::new();
    let mut prev_was_bracket_or_comma = true;

    for token in tokens {
        match token.token_type {
            JsonTokenType::String => {
                if prev_was_bracket_or_comma {
                    result.push((token.text, theme.json_key));
                } else {
                    result.push((token.text, theme.json_string));
                }
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Number => {
                result.push((token.text, theme.json_number));
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Boolean => {
                result.push((token.text, theme.json_boolean));
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Null => {
                result.push((token.text, theme.json_null));
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Bracket => {
                result.push((token.text, theme.json_bracket));
                prev_was_bracket_or_comma = true;
            }
            JsonTokenType::Comma => {
                result.push((token.text, theme.json_bracket));
                prev_was_bracket_or_comma = true;
            }
            JsonTokenType::Colon => {
                result.push((token.text, theme.json_bracket));
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Plain => {
                result.push((token.text, theme.json_bracket));
                prev_was_bracket_or_comma = false;
            }
            JsonTokenType::Key => {
                // 不应该到达这里
                result.push((token.text, theme.json_key));
                prev_was_bracket_or_comma = false;
            }
        }
    }

    result
}

/// 检查字符串是否为有效的 JSON
pub fn is_valid_json(s: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(s).is_ok()
}
