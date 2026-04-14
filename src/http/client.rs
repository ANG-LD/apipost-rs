//! HTTP客户端模块
//!
//! 负责发送HTTP请求并处理响应
//! 支持各种HTTP方法和配置选项

use crate::app::environment::EnvironmentManager;
use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method, Proxy,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use reqwest::multipart;

/// HTTP客户端管理器
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
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(Self {
            client,
            env_manager,
        })
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

        // 构建请求头
        let headers = self.build_headers(&request.headers)?;

        // 替换请求体中的环境变量
        let body = request.body.as_ref().map(|b| {
            self.env_manager.replace_variables(b)
        });

        // 解析HTTP方法
        let method = Method::try_from(request.method.to_uppercase().as_str())
            .with_context(|| format!("无效的HTTP方法: {}", request.method))?;

        // 根据设置决定使用哪个客户端
        let use_custom_client = options.timeout_secs != 30 || !options.follow_redirects || !options.verify_ssl;

        let response = if use_custom_client {
            // 构建自定义客户端
            let mut builder = Client::builder()
                .timeout(Duration::from_secs(options.timeout_secs))
                .redirect(if options.follow_redirects { reqwest::redirect::Policy::default() } else { reqwest::redirect::Policy::none() });

            if !options.verify_ssl {
                builder = builder.danger_accept_invalid_certs(true);
            }

            let custom_client = builder.build().context("创建自定义HTTP客户端失败")?;

            custom_client
                .request(method, &url)
                .headers(headers)
                .multipart(self.build_multipart(request)?)
                .send()
                .await
                .context("请求发送失败")?
        } else {
            self.client
                .request(method, &url)
                .headers(headers)
                .multipart(self.build_multipart(request)?)
                .send()
                .await
                .context("请求发送失败")?
        };

        let elapsed = start_time.elapsed();
        let status = response.status().as_u16();
        let response_headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_bytes = response
            .bytes()
            .await
            .context("读取响应体失败")?;
        let body_text = String::from_utf8_lossy(&body_bytes).to_string();

        Ok(HttpResponse {
            status,
            headers: response_headers,
            body: body_text,
            time_ms: elapsed.as_millis() as i64,
            size_bytes: body_bytes.len() as i64,
        })
    }

    /// 构建HTTP请求头
    fn build_headers(&self, headers: &[(String, String)]) -> Result<HeaderMap> {
        let mut header_map = HeaderMap::new();

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
        let mut form = multipart::Form::new();

        // 添加文本字段
        for (name, value) in &request.text_fields {
            let value = self.env_manager.replace_variables(value);
            form = form.text(name.clone(), value);
        }

        // 添加文件字段
        for file_field in &request.file_fields {
            let file_path = file_field.file_path.clone();
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
        // 尝试解析并格式化JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.body) {
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| self.body.clone())
        } else {
            self.body.clone()
        }
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
        };
        assert_eq!(response.status_text(), "OK");
        assert!(response.is_success());

        let response = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: String::new(),
            time_ms: 100,
            size_bytes: 0,
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
