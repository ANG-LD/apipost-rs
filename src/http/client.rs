//! HTTP客户端模块
//!
//! 负责发送HTTP请求并处理响应
//! 支持各种HTTP方法和配置选项

use crate::app::environment::EnvironmentManager;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, ACCEPT_ENCODING},
    Client, Method, Proxy,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};
use reqwest::multipart;

/// HTTP客户端管理器
#[derive(Clone)]
pub struct HttpClient {
    /// reqwest客户端
    client: Client,
    /// 环境变量管理器引用
    env_manager: EnvironmentManager,
}

/// 请求设置
#[derive(Debug, Clone)]
pub struct RequestOptions {
    /// 超时秒数
    pub timeout_secs: u64,
    /// 重试次数
    pub retry_count: u32,
    /// 跟随重定向
    pub follow_redirects: bool,
    /// 验证 SSL
    pub verify_ssl: bool,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            retry_count: 0,
            follow_redirects: true,
            verify_ssl: true,
        }
    }
}

impl HttpClient {
    /// 创建新的HTTP客户端
    pub fn new(env_manager: EnvironmentManager) -> Result<Self> {
        let client = Client::builder()
            .no_brotli()
            .no_gzip()
            .no_deflate()
            .timeout(Duration::from_secs(30))
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(Self {
            client,
            env_manager,
        })
    }

    /// 创建带有自定义超时的HTTP客户端
    pub fn with_timeout(timeout_secs: u64, env_manager: EnvironmentManager) -> Result<Self> {
        let client = Client::builder()
            .no_brotli()
            .no_gzip()
            .no_deflate()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(Self {
            client,
            env_manager,
        })
    }

    /// 创建带有代理的HTTP客户端
    pub fn with_proxy(proxy_url: &str, env_manager: EnvironmentManager) -> Result<Self> {
        let proxy = Proxy::all(proxy_url).context("代理URL无效")?;
        let client = Client::builder()
            .no_brotli()
            .no_gzip()
            .no_deflate()
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(Self {
            client,
            env_manager,
        })
    }

    /// Update the environment manager (called when active environment changes)
    pub fn set_env_manager(&mut self, env_manager: EnvironmentManager) {
        self.env_manager = env_manager;
    }

    /// 发送HTTP请求
    pub async fn send_request(&self, request: &HttpRequest) -> Result<HttpResponse> {
        self.send_request_with_settings(request, RequestOptions::default()).await
    }

    /// 发送HTTP请求（带设置）
    pub async fn send_request_with_settings(&self, request: &HttpRequest, options: RequestOptions) -> Result<HttpResponse> {
        let start_time = Instant::now();

        // 替换URL中的环境变量
        let url = self.env_manager.replace_variables(&request.url);
        log::debug!("=== HTTP请求详情 ===");
        log::debug!("方法: {}", request.method);
        log::debug!("原始URL: {}", request.url);
        if request.url != url {
            log::debug!("替换后URL: {}", url);
        }

        // 构建请求头
        let headers = self.build_headers(&request.headers)?;

        // 替换请求体中的环境变量
        let body = request.body.as_ref().map(|b| {
            let replaced = self.env_manager.replace_variables(b);
            if b != &replaced {
                log::debug!("请求体已替换环境变量 (原始长度: {}, 替换后长度: {})", b.len(), replaced.len());
            }
            replaced
        });

        log::debug!("请求头 ({} 项):", request.headers.len());
        for (name, value) in &request.headers {
            let resolved = self.env_manager.replace_variables(value);
            if *value != resolved {
                log::debug!("  {}: {} -> {}", name, value, resolved);
            } else {
                log::debug!("  {}: {}", name, value);
            }
        }
        if let Some(ref body_content) = body {
            let preview = if body_content.len() > 500 {
                format!("{}...(截断, 总长度: {})", &body_content[..500], body_content.len())
            } else {
                body_content.clone()
            };
            log::debug!("请求体: {}", preview);
        }
        log::debug!("===================");

        // 解析HTTP方法
        let method = Method::try_from(request.method.to_uppercase().as_str())
            .with_context(|| format!("无效的HTTP方法: {}", request.method))?;

        // 根据设置决定使用哪个客户端
        let use_custom_client = options.timeout_secs != 30 || !options.follow_redirects || !options.verify_ssl;

        log::debug!("开始发送请求，使用自定义客户端: {}", use_custom_client);

let response = if use_custom_client {
            // 构建自定义客户端
            let mut builder = Client::builder()
                .no_brotli()
                .no_gzip()
                .no_deflate()
                .timeout(Duration::from_secs(options.timeout_secs))
                .redirect(if options.follow_redirects {
                    reqwest::redirect::Policy::default()
                } else {
                    reqwest::redirect::Policy::none()
                });

            if !options.verify_ssl {
                builder = builder.danger_accept_invalid_certs(true);
            }

            let custom_client = builder.build().context("创建自定义HTTP客户端失败")?;

            let request_builder = custom_client
                .request(method, &url)
                .headers(headers);

            // 根据是否有请求体决定发送方式
            let request_builder = if let Some(ref body_content) = body {
                request_builder.body(body_content.clone())
            } else {
                request_builder.multipart(self.build_multipart(request)?)
            };

            request_builder
                .send()
                .await
                .context("请求发送失败")?
        } else {
            let request_builder = self.client
                .request(method, &url)
                .headers(headers);

            // 根据是否有请求体决定发送方式
            let request_builder = if let Some(ref body_content) = body {
                request_builder.body(body_content.clone())
            } else {
                request_builder.multipart(self.build_multipart(request)?)
            };

            request_builder
                .send()
                .await
                .context("请求发送失败")?
        };

        let elapsed = start_time.elapsed();
        let status = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // 解析Set-Cookie头
        let mut cookies = Vec::new();
        for (name, value) in &response_headers {
            if name.to_lowercase() == "set-cookie" {
                if let Some(cookie) = Cookie::from_set_cookie_header(value) {
                    cookies.push(cookie);
                }
            }
        }

        let body_bytes = response
            .bytes()
            .await
            .context("读取响应体失败")?;

        // 记录网络传输的压缩大小
        let compressed_size = body_bytes.len() as i64;

        // 如果是 gzip 压缩数据，先解压用于渲染
        let is_gzip = body_bytes.len() >= 2 && body_bytes[0] == 0x1f && body_bytes[1] == 0x8b;
        let body_text = if is_gzip {
            let mut decoder = GzDecoder::new(&body_bytes[..]);
            let mut decompressed = Vec::new();
            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => String::from_utf8_lossy(&decompressed).to_string(),
                Err(_) => String::from_utf8_lossy(&body_bytes).to_string(),
            }
        } else {
            String::from_utf8_lossy(&body_bytes).to_string()
        };

        Ok(HttpResponse {
            status,
            headers: response_headers,
            body: body_text,
            time_ms: elapsed.as_millis() as i64,
            size_bytes: compressed_size,
            cookies,
        })
    }

    /// 构建HTTP请求头
    fn build_headers(&self, headers: &[(String, String)]) -> Result<HeaderMap> {
        let mut header_map = HeaderMap::new();

        // 添加 Accept-Encoding 以接收 gzip 压缩数据（如果用户没有自定义）
        if !headers.iter().any(|(k, _)| k.to_lowercase() == "accept-encoding") {
            header_map.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        }

        for (name, value) in headers {
            // 替换头部值中的环境变量
            let value = self.env_manager.replace_variables(value);

            let header_name = HeaderName::try_from(name.as_str())
                .with_context(|| format!("无效的请求头名称: {}", name))?;
            let header_value = HeaderValue::from_str(&value)
                .with_context(|| format!("无效的请求头值: {}", value))?;

            header_map.insert(header_name, header_value);
        }

        Ok(header_map)
    }

    /// 构建multipart表单
    fn build_multipart(&self, request: &HttpRequest) -> Result<multipart::Form> {
        log::debug!("构建multipart表单: text_fields={}, file_fields={}",
            request.text_fields.len(), request.file_fields.len());

        let mut form = multipart::Form::new();

        // 添加文本字段
        for (name, value) in &request.text_fields {
            let value = self.env_manager.replace_variables(value);
            log::debug!("添加文本字段: {}={}", name, value);
            form = form.text(name.clone(), value);
        }

        // 添加文件字段
        for file_field in &request.file_fields {
            let file_path = file_field.file_path.clone();
            log::debug!("添加文件字段: {} -> {}", file_field.field_name, file_path);
            if std::path::Path::new(&file_path).exists() {
                let file_name = std::path::Path::new(&file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string());

                // 读取文件内容
                let file_content = std::fs::read(&file_path)
                    .map_err(|e| anyhow::anyhow!("无法读取文件 {}: {}", file_path, e))?;

                let part = multipart::Part::bytes(file_content)
                    .file_name(file_name)
                    .mime_str(&file_field.content_type)
                    .map_err(|e| anyhow::anyhow!("无法创建文件部分: {}", e))?;

                form = form.part(file_field.field_name.clone(), part);
            } else {
                log::warn!("文件不存在: {}", file_path);
            }
        }

        Ok(form)
    }
}

/// HTTP请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    /// HTTP方法
    pub method: String,
    /// 请求URL
    pub url: String,
    /// 请求头列表
    pub headers: Vec<(String, String)>,
    /// 请求体
    pub body: Option<String>,
    /// 内容类型
    pub content_type: Option<String>,
    /// 文本字段 (for multipart)
    pub text_fields: Vec<(String, String)>,
    /// 文件字段 (for multipart)
    pub file_fields: Vec<FileField>,
}

/// 文件字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileField {
    /// 字段名
    pub field_name: String,
    /// 文件路径
    pub file_path: String,
    /// 内容类型
    pub content_type: String,
}

impl HttpRequest {
    /// 创建新的HTTP请求
    pub fn new(method: Method, url: String) -> Self {
        Self {
            method: method.to_string(),
            url,
            headers: Vec::new(),
            body: None,
            content_type: None,
            text_fields: Vec::new(),
            file_fields: Vec::new(),
        }
    }

    /// 添加请求头
    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.push((name, value));
        self
    }

    /// 设置请求体
    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    /// 设置JSON内容类型
    pub fn with_json_content_type(self) -> Self {
        self.with_header("Content-Type".to_string(), "application/json".to_string())
    }

    /// 从方法字符串创建请求
    pub fn from_method_str(method: &str, url: String) -> Result<Self> {
        let _ = Method::try_from(method.to_uppercase().as_str())
            .with_context(|| format!("无效的HTTP方法: {}", method))?;
        Ok(Self {
            method: method.to_uppercase(),
            url,
            headers: Vec::new(),
            body: None,
            content_type: None,
            text_fields: Vec::new(),
            file_fields: Vec::new(),
        })
    }

    /// 获取方法引用
    pub fn method(&self) -> &str {
        &self.method
    }
}

/// Cookie结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    /// Cookie名称
    pub name: String,
    /// Cookie值
    pub value: String,
    /// 域名
    pub domain: Option<String>,
    /// 路径
    pub path: Option<String>,
    /// 过期时间
    pub expires: Option<String>,
    /// 是否仅HTTP
    pub http_only: bool,
    /// 是否安全
    pub secure: bool,
}

impl Cookie {
    /// 从Set-Cookie头解析Cookie
    pub fn from_set_cookie_header(header_value: &str) -> Option<Self> {
        let parts: Vec<&str> = header_value.split(';').collect();
        if parts.is_empty() {
            return None;
        }

        // 第一个部分是 name=value
        let name_value = parts[0].trim();
        let eq_pos = name_value.find('=')?;
        let name = name_value[..eq_pos].trim().to_string();
        let value = name_value[eq_pos + 1..].trim().to_string();

        let mut domain = None;
        let mut path = None;
        let mut expires = None;
        let mut http_only = false;
        let mut secure = false;

        for part in parts.iter().skip(1) {
            let part = part.trim().to_lowercase();
            if part.starts_with("domain=") {
                domain = Some(part[7..].to_string());
            } else if part.starts_with("path=") {
                path = Some(part[5..].to_string());
            } else if part.starts_with("expires=") {
                expires = Some(part[8..].to_string());
            } else if part == "httponly" {
                http_only = true;
            } else if part == "secure" {
                secure = true;
            }
        }

        Some(Cookie {
            name,
            value,
            domain,
            path,
            expires,
            http_only,
            secure,
        })
    }
}

/// HTTP响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP状态码
    pub status: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: String,
    /// 响应时间（毫秒）
    pub time_ms: i64,
    /// 响应大小（字节）
    pub size_bytes: i64,
    /// Cookie列表
    pub cookies: Vec<Cookie>,
}

impl HttpResponse {
    /// 判断响应是否成功（2xx状态码）
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 获取状态文本
    pub fn status_text(&self) -> &'static str {
        match self.status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => "Unknown",
        }
    }

    /// 尝试解析响应体为JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        serde_json::from_str(&self.body).ok()
    }

    /// 格式化响应体（如果可以）
    pub fn format_body(&self) -> String {
        // 使用带折叠的 JSON 格式化
        format_json_folded(&self.body, 5, 2)
    }

    /// 检测内容类型
    pub fn detect_content_type(&self) -> Option<String> {
        self.headers
            .get("content-type")
            .or_else(|| self.headers.get("Content-Type"))
            .cloned()
    }
}

/// 带折叠的 JSON 格式化
pub fn format_json_folded(json_str: &str, max_depth: usize, indent_size: usize) -> String {
    let indent = |d: usize| " ".repeat(d * indent_size);

    fn format_value(value: &serde_json::Value, current_depth: usize, max_depth: usize, indent_size: usize) -> String {
        let ind = " ".repeat(current_depth * indent_size);
        let next_ind = " ".repeat((current_depth + 1) * indent_size);

        match value {
            serde_json::Value::Object(map) => {
                if map.is_empty() {
                    return "{}".to_string();
                }
                if current_depth >= max_depth {
                    return format!("{{ {} keys... }}", map.len());
                }
                let mut s = String::from("{\n");
                for (i, (k, v)) in map.iter().enumerate() {
                    let comma = if i < map.len() - 1 { "," } else { "" };
                    s.push_str(&format!("{next_ind}\"{k}\": {}", format_value(v, current_depth + 1, max_depth, indent_size)));
                    s.push_str(comma);
                    s.push('\n');
                }
                s.push_str(&ind);
                s.push('}');
                s
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    return "[]".to_string();
                }
                if current_depth >= max_depth {
                    return format!("[ {} items... ]", arr.len());
                }
                let mut s = String::from("[\n");
                for (i, v) in arr.iter().enumerate() {
                    let comma = if i < arr.len() - 1 { "," } else { "" };
                    s.push_str(&format!("{next_ind}{}", format_value(v, current_depth + 1, max_depth, indent_size)));
                    s.push_str(comma);
                    s.push('\n');
                }
                s.push_str(&ind);
                s.push(']');
                s
            }
            serde_json::Value::String(st) => format!("\"{}\"", escape_json_str(st)),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
        }
    }

    fn escape_json_str(s: &str) -> String {
        let mut r = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => r.push_str("\\\""),
                '\\' => r.push_str("\\\\"),
                '\n' => r.push_str("\\n"),
                '\r' => r.push_str("\\r"),
                '\t' => r.push_str("\\t"),
                ch if ch.is_control() => r.push_str(&format!("\\u{:04x}", ch as u32)),
                ch => r.push(ch),
            }
        }
        r
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        format_value(&value, 0, max_depth, indent_size)
    } else {
        json_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_env_manager() -> EnvironmentManager {
        EnvironmentManager::new()
    }

    #[test]
    fn test_http_request_creation() {
        let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "https://api.example.com");
        assert!(request.headers.is_empty());
        assert!(request.body.is_none());
    }

    #[test]
    fn test_http_request_with_headers() {
        let request = HttpRequest::new(Method::POST, "https://api.example.com".to_string())
            .with_header("Content-Type".to_string(), "application/json".to_string())
            .with_header("Authorization".to_string(), "Bearer token123".to_string());

        assert_eq!(request.headers.len(), 2);
    }

    #[test]
    fn test_http_request_with_body() {
        let request = HttpRequest::new(Method::POST, "https://api.example.com".to_string())
            .with_json_content_type()
            .with_body(r#"{"name":"test"}"#.to_string());

        assert!(request.body.is_some());
        assert_eq!(request.body.unwrap(), r#"{"name":"test"}"#);
    }

    #[test]
    fn test_http_response_status_text() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
            time_ms: 100,
            size_bytes: 0,
            cookies: Vec::new(),
        };
        assert_eq!(response.status_text(), "OK");
        assert!(response.is_success());

        let response = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: String::new(),
            time_ms: 100,
            size_bytes: 0,
            cookies: Vec::new(),
        };
        assert_eq!(response.status_text(), "Not Found");
        assert!(!response.is_success());
    }

    #[test]
    fn test_http_response_format_body() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: r#"{"name":"test","value":123}"#.to_string(),
            time_ms: 100,
            size_bytes: 0,
            cookies: Vec::new(),
        };

        let formatted = response.format_body();
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("test"));
    }

    #[tokio::test]
    async fn test_send_simple_request() {
        let env_manager = create_mock_env_manager();
        let client = HttpClient::new(env_manager).unwrap();

        let request = HttpRequest::new(Method::GET, "https://httpbin.org/get".to_string());
        let response = client.send_request(&request).await;

        // 由于网络问题可能失败，我们只检查不panic
        if response.is_ok() {
            let resp = response.unwrap();
            assert_eq!(resp.status, 200);
        }
    }
}
