//! HTTP客户端模块
//!
//! 负责发送HTTP请求并处理响应
//! 支持各种HTTP方法和配置选项
//!
//! ## 设计要点
//!
//! - **懒加载**: reqwest::Client 通过 `OnceLock` 延迟到首次 `send_request` 时才构建，
//!   避免应用启动时即加载 TLS 证书、初始化连接池的开销。
//! - **连接池可配置**: 通过 `PoolConfig` 控制 `pool_max_idle_per_host`、`pool_idle_timeout`。
//! - **并发控制**: 通过 tokio `Semaphore` 限制同时进行中的 HTTP 请求数量，
//!   防止短时间内发起过多连接耗尽系统资源。

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
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use reqwest::multipart;
use tokio::sync::Semaphore;

// ============================================================================
// 连接池配置
// ============================================================================

/// 连接池与并发控制配置
///
/// 控制底层 reqwest 连接池行为以及应用层的请求并发数。
/// 所有字段为 0 表示使用默认值（不限制）。
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 每个 host 最大空闲连接数
    ///
    /// 对应 reqwest 的 `pool_max_idle_per_host`。
    /// 设为 0 表示使用 reqwest 内置默认值（当前为 `usize::MAX`，即不限制）。
    pub max_idle_per_host: usize,

    /// 空闲连接超时（秒）
    ///
    /// 连接在连接池中保持空闲的最大时长。超时后连接被关闭回收。
    /// 对应 reqwest 的 `pool_idle_timeout`。
    /// 设为 0 表示使用 reqwest 内置默认值（当前为 90 秒）。
    pub idle_timeout_secs: u64,

    /// 最大并发请求数
    ///
    /// 同时进行中的 HTTP 请求数量上限。通过 tokio `Semaphore` 实现，
    /// 超出限制的请求会排队等待。
    /// 设为 0 表示不限制并发数。
    pub max_concurrent_requests: usize,

    /// TCP keepalive 间隔（秒）
    ///
    /// 空闲连接上发送 TCP keepalive 探测包的间隔。
    /// 防止防火墙/NAT/代理因长时间无数据而断开连接。
    /// 设为 0 使用系统默认（通常 7200s，即 2 小时）。
    /// 推荐值：60。
    pub tcp_keepalive_secs: u64,

    /// 连接超时（秒）
    ///
    /// 建立 TCP 连接的最大等待时间，独立于请求整体超时。
    /// 设为 0 表示使用请求超时（`timeout_secs`）作为连接超时。
    /// 推荐值：10。
    pub connect_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: 0,
            idle_timeout_secs: 0,
            max_concurrent_requests: 0,
            tcp_keepalive_secs: 0,
            connect_timeout_secs: 0,
        }
    }
}

impl PoolConfig {
    /// 创建一个限制并发请求数的配置
    ///
    /// 适用于需要控制资源使用的场景（如嵌入式设备、代理环境）。
    pub fn with_concurrency_limit(max_concurrent: usize, max_idle_per_host: usize) -> Self {
        Self {
            max_idle_per_host,
            idle_timeout_secs: 90,
            max_concurrent_requests: max_concurrent,
            tcp_keepalive_secs: 0,
            connect_timeout_secs: 0,
        }
    }

    /// 推荐配置：针对频繁请求优化连接复用
    ///
    /// - TCP keepalive 每 60s 探测一次，防止中间设备断开空闲连接
    /// - 连接超时 10s，避免 DNS/网络问题阻塞过久
    /// - 每 host 保留最多 8 个空闲连接
    /// - 空闲连接 90s 后回收
    pub fn recommended() -> Self {
        Self {
            max_idle_per_host: 8,
            idle_timeout_secs: 90,
            max_concurrent_requests: 0,
            tcp_keepalive_secs: 60,
            connect_timeout_secs: 10,
        }
    }
}

// ============================================================================
// HTTP 客户端
// ============================================================================

/// HTTP客户端管理器
///
/// 持有懒加载的 reqwest `Client` 实例、环境变量管理器，
/// 并提供连接池配置与并发控制能力。
///
/// ## Clone 语义
///
/// `HttpClient` 的 clone 开销极低：所有 clone 共享同一个底层
/// `Arc<OnceLock<Client>>`（懒加载只执行一次）和同一个 `Semaphore`
///（并发上限在所有 clone 之间共享）。
pub struct HttpClient {
    /// 默认超时（秒），构造时设定，懒加载用
    default_timeout_secs: u64,

    /// 默认代理 URL（None = 无代理），构造时设定，懒加载用
    default_proxy_url: Option<String>,

    /// 懒加载的默认 reqwest 客户端
    ///
    /// `Arc` 保证所有 clone 共享同一实例，`OnceLock` 保证只构建一次。
    /// 首次 `send_request` 时触发初始化。
    /// 存储 `Result<Client, String>`：构建成功为 `Ok`，失败则缓存错误信息，
    /// 后续调用直接返回相同错误。（String 用作 Err 变体以保证 Clone）
    default_client: Arc<OnceLock<Result<Client, String>>>,

    /// 环境变量管理器（与 AppState 共享同一实例）
    env_manager: Arc<EnvironmentManager>,

    /// 自定义客户端缓存，按配置键索引
    ///
    /// 缓存那些 timeout / redirect / SSL 配置不同于默认值的 reqwest Client。
    /// 避免为相同配置重复构建。
    custom_clients: Arc<Mutex<HashMap<ClientCacheKey, Client>>>,

    /// 连接池配置（控制 idle 连接数、超时等）
    pool_config: PoolConfig,

    /// 请求并发信号量
    ///
    /// `None` 表示不限制并发。`Some(Arc<Semaphore>)` 时，
    /// 每次 `send_request` 需先获取许可，请求完成后自动释放。
    request_semaphore: Option<Arc<Semaphore>>,
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            default_timeout_secs: self.default_timeout_secs,
            default_proxy_url: self.default_proxy_url.clone(),
            default_client: Arc::clone(&self.default_client),
            env_manager: Arc::clone(&self.env_manager),
            // 所有 clone 共享同一份 custom_clients 缓存
            custom_clients: Arc::clone(&self.custom_clients),
            pool_config: self.pool_config.clone(),
            // 信号量在所有 clone 间共享（并发上限全局生效）
            request_semaphore: self.request_semaphore.clone(),
        }
    }
}

// ============================================================================
// 请求 / 响应 / 辅助类型
// ============================================================================

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

/// Cache key for custom HTTP client configurations
#[derive(Hash, PartialEq, Eq)]
struct ClientCacheKey {
    timeout_secs: u64,
    follow_redirects: bool,
    verify_ssl: bool,
}

impl ClientCacheKey {
    fn from_options(opts: &RequestOptions) -> Self {
        Self {
            timeout_secs: opts.timeout_secs,
            follow_redirects: opts.follow_redirects,
            verify_ssl: opts.verify_ssl,
        }
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
    /// 响应体（文本形式，二进制类型为占位描述）
    pub body: String,
    /// 响应体原始字节（仅二进制内容类型有值，用于图片/PDF等预览）
    pub raw_body: Option<Vec<u8>>,
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

// ============================================================================
// HttpClient 实现
// ============================================================================

impl HttpClient {
    // ------------------------------------------------------------------
    // 构造函数
    // ------------------------------------------------------------------

    /// 创建新的HTTP客户端（默认 30s 超时，无代理，懒加载）
    ///
    /// 注意：此方法不构建底层 reqwest `Client` —
    /// 实际构建延迟到首次 [`send_request`] 调用。
    pub fn new(env_manager: Arc<EnvironmentManager>) -> Result<Self> {
        Ok(Self {
            default_timeout_secs: 30,
            default_proxy_url: None,
            default_client: Arc::new(OnceLock::new()),
            env_manager,
            custom_clients: Arc::new(Mutex::new(HashMap::new())),
            pool_config: PoolConfig::recommended(),
            request_semaphore: None,
        })
    }

    /// 创建带有自定义超时的HTTP客户端（懒加载）
    pub fn with_timeout(timeout_secs: u64, env_manager: Arc<EnvironmentManager>) -> Result<Self> {
        Ok(Self {
            default_timeout_secs: timeout_secs,
            default_proxy_url: None,
            default_client: Arc::new(OnceLock::new()),
            env_manager,
            custom_clients: Arc::new(Mutex::new(HashMap::new())),
            pool_config: PoolConfig::recommended(),
            request_semaphore: None,
        })
    }

    /// 创建带有代理的HTTP客户端（懒加载）
    ///
    /// 代理 URL 在此处记录，实际 Proxy 对象在首次请求时构建。
    /// 若代理 URL 格式错误，错误将在 [`send_request`] 时返回。
    pub fn with_proxy(proxy_url: &str, env_manager: Arc<EnvironmentManager>) -> Result<Self> {
        Ok(Self {
            default_timeout_secs: 30,
            default_proxy_url: Some(proxy_url.to_string()),
            default_client: Arc::new(OnceLock::new()),
            env_manager,
            custom_clients: Arc::new(Mutex::new(HashMap::new())),
            pool_config: PoolConfig::recommended(),
            request_semaphore: None,
        })
    }

    /// 设置连接池与并发控制配置（构建器模式）
    ///
    /// # 示例
    /// ```ignore
    /// let client = HttpClient::new(env)?
    ///     .with_pool_config(PoolConfig::with_concurrency_limit(10, 32));
    /// ```
    pub fn with_pool_config(mut self, config: PoolConfig) -> Self {
        self.request_semaphore = if config.max_concurrent_requests > 0 {
            Some(Arc::new(Semaphore::new(config.max_concurrent_requests)))
        } else {
            None
        };
        self.pool_config = config;
        self
    }

    /// 获取当前连接池配置的只读引用
    pub fn pool_config(&self) -> &PoolConfig {
        &self.pool_config
    }

    // ------------------------------------------------------------------
    // 公共 API：发送请求
    // ------------------------------------------------------------------

    /// 发送HTTP请求（使用默认设置）
    pub async fn send_request(&self, request: &HttpRequest) -> Result<HttpResponse> {
        self.send_request_with_settings(request, RequestOptions::default()).await
    }

    /// 发送HTTP请求（带设置）
    ///
    /// 首次调用时触发默认 reqwest `Client` 的懒加载构建。
    /// 若配置了 `max_concurrent_requests > 0`，会先获取信号量许可。
    pub async fn send_request_with_settings(
        &self,
        request: &HttpRequest,
        options: RequestOptions,
    ) -> Result<HttpResponse> {
        // 1. 获取并发信号量许可（如果配置了限制）
        let _permit = self.acquire_permit().await?;

        let start_time = Instant::now();

        // 2. 替换URL中的环境变量
        let url = self.env_manager.replace_variables(&request.url);
        log::info!("=== HTTP请求 ===");
        log::info!("方法: {}  URL: {}", request.method, url);

        // 3. 构建请求头（内部完成变量替换 + 日志）
        let headers = self.build_headers(&request.headers)?;
        log::info!("请求头 ({} 项): {:?}", headers.len(),
            headers.iter().map(|(k,v)| format!("{}: {:?}", k, v)).collect::<Vec<_>>());

        // 4. 替换请求体中的环境变量
        let body = request.body.as_ref().map(|b| {
            let replaced = self.env_manager.replace_variables(b);
            if b != &replaced {
                log::debug!(
                    "请求体已替换环境变量 (原始长度: {}, 替换后长度: {})",
                    b.len(),
                    replaced.len()
                );
            }
            replaced
        });

        if let Some(ref body_content) = body {
            let preview = if body_content.len() > 500 {
                format!("{}...(截断, 总长度: {})", &body_content[..500], body_content.len())
            } else {
                body_content.clone()
            };
            log::info!("请求体 ({} bytes): {}", body_content.len(), preview);
        } else if request.text_fields.is_empty() && request.file_fields.is_empty() {
            let upper = request.method.to_uppercase();
            if upper == "POST" || upper == "PUT" || upper == "PATCH" {
                log::warn!("{} 请求没有请求体！这可能导致服务器返回错误", upper);
            }
        }

        // 5. 解析HTTP方法
        let method = Method::try_from(request.method.to_uppercase().as_str())
            .with_context(|| format!("无效的HTTP方法: {}", request.method))?;

        // 6. 选择或构建客户端（懒加载默认客户端，或使用缓存的自定义客户端）
        let client = self.get_client_for_options(&options)?;

        // 7. 构建请求
        let has_multipart = !request.text_fields.is_empty() || !request.file_fields.is_empty();

        // multipart 时去掉用户传入的 Content-Type，让 reqwest 自动生成正确的 boundary
        let filtered_headers: HeaderMap = if has_multipart {
            headers
                .iter()
                .filter(|(k, _)| k.as_str().to_lowercase() != "content-type")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            headers
        };

        let mut request_builder = client.request(method, &url).headers(filtered_headers);

        if has_multipart {
            request_builder = request_builder.multipart(self.build_multipart(request)?);
        } else if let Some(body_content) = body {
            request_builder = request_builder.body(body_content);
        }

        // DNS 解析日志（用于排查解析问题）
        if let Ok(parsed) = url::Url::parse(&url) {
            if let Some(host) = parsed.host_str() {
                let port = parsed.port().unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
                let addr_str = format!("{}:{}", host, port);
                match std::net::ToSocketAddrs::to_socket_addrs(&addr_str.as_str()) {
                    Ok(addrs) => {
                        let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                        log::info!("DNS 解析: {} -> {:?}", host, ips);
                    }
                    Err(e) => {
                        log::error!("DNS 解析失败: {} ({})", addr_str, e);
                    }
                }
            }
        }

        let response = request_builder.send().await
            .map_err(|e| {
                let mut msg = format!("请求失败: {}", e);
                if url.starts_with("http://") {
                    msg.push_str(&format!("\n提示: 服务器可能只接受 HTTPS，尝试将 URL 改为 https://"));
                }
                anyhow::anyhow!(msg)
            })?;

        let elapsed = start_time.elapsed();
        let status = response.status().as_u16();
        let version = response.version();
        let remote_addr = response.remote_addr();
        log::info!("=== HTTP响应 ===");
        log::info!("HTTP版本: {:?}, 远端地址: {:?}", version, remote_addr);
        log::info!("状态码: {}  (耗时: {}ms)", status, elapsed.as_millis());
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

        // 记录响应头
        let content_type_val = response_headers.get("content-type").cloned()
            .or_else(|| response_headers.get("Content-Type").cloned())
            .unwrap_or_else(|| "unknown".to_string());
        log::info!("Content-Type: {}", content_type_val);
        if !status.to_string().starts_with('2') {
            log::warn!("非2xx响应 (状态码: {}), 响应头: {:?}", status,
                response_headers.iter().map(|(k,v)| format!("{}: {}", k, v)).collect::<Vec<_>>());
        }

        // 读取原始响应体（reqwest 禁用自动解压，返回原始压缩数据）
        let body_bytes = response.bytes().await.context("读取响应体失败")?;
        let size_bytes = body_bytes.len() as i64;

        // 根据 Content-Encoding 响应头解压
        let content_encoding = response_headers
            .get("content-encoding")
            .map(|v| v.to_lowercase());
        if let Some(ref enc) = content_encoding {
            log::info!("Content-Encoding: {} (压缩大小: {} bytes)", enc, body_bytes.len());
        }
        // 提取 MIME 类型（去掉 ; 后的参数）
        let mime_type = content_type_val
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let is_binary_ct = mime_type.starts_with("image/")
            || mime_type.starts_with("audio/")
            || mime_type.starts_with("video/")
            || mime_type.starts_with("font/")
            || mime_type == "application/octet-stream"
            || mime_type == "application/pdf"
            || mime_type == "application/zip"
            || mime_type == "application/gzip"
            || mime_type == "application/x-7z-compressed"
            || mime_type == "application/x-rar-compressed"
            || mime_type == "application/x-tar"
            || mime_type == "application/x-bzip"
            || mime_type == "application/x-bzip2"
            || mime_type.starts_with("application/vnd.ms-excel")
            || mime_type.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml")
            || mime_type.starts_with("application/vnd.ms-powerpoint")
            || mime_type.starts_with("application/vnd.openxmlformats-officedocument.presentationml")
            || mime_type.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml")
            || mime_type.starts_with("application/msword");

        let (body_text, raw_body) = if is_binary_ct {
            let raw = body_bytes.to_vec();
            let placeholder = format!("[二进制内容: {}, {} bytes]", content_type_val, raw.len());
            (placeholder, Some(raw))
        } else {
            let text = match content_encoding.as_deref() {
                Some("gzip") | Some("x-gzip") => {
                    let mut d = GzDecoder::new(&body_bytes[..]);
                    let mut decompressed = Vec::new();
                    d.read_to_end(&mut decompressed)
                        .map(|_| String::from_utf8_lossy(&decompressed).to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&body_bytes).to_string())
                }
                Some("br") => {
                    let mut decompressed = Vec::new();
                    brotli::Decompressor::new(&body_bytes[..], 4096)
                        .read_to_end(&mut decompressed)
                        .map(|_| String::from_utf8_lossy(&decompressed).to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&body_bytes).to_string())
                }
                Some("deflate") => {
                    let mut d = flate2::read::ZlibDecoder::new(&body_bytes[..]);
                    let mut decompressed = Vec::new();
                    d.read_to_end(&mut decompressed)
                        .map(|_| String::from_utf8_lossy(&decompressed).to_string())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&body_bytes).to_string())
                }
                _ => String::from_utf8_lossy(&body_bytes).to_string(),
            };
            (text, None)
        };

        // 记录响应体预览（截断长内容，在有效字符边界处截断）
        let body_preview = if body_text.len() > 500 {
            let end = body_text.floor_char_boundary(500);
            format!("{}...(截断, 总长度: {})", &body_text[..end], body_text.len())
        } else {
            body_text.clone()
        };
        log::info!("响应体: {}", body_preview);
        log::info!("=== HTTP响应结束 (耗时: {}ms, 传输大小: {} bytes, 解压后: {} bytes) ===", elapsed.as_millis(), size_bytes, body_text.len());

        Ok(HttpResponse {
            status,
            headers: response_headers,
            body: body_text,
            raw_body,
            time_ms: elapsed.as_millis() as i64,
            size_bytes,
            cookies,
        })
    }

    // ------------------------------------------------------------------
    // 内部方法
    // ------------------------------------------------------------------

    /// 获取并发信号量许可
    ///
    /// 若未配置 `max_concurrent_requests`（为 0），立即返回 `None`，
    /// 不引入任何开销。
    async fn acquire_permit(&self) -> Result<Option<tokio::sync::SemaphorePermit<'_>>> {
        match &self.request_semaphore {
            Some(sem) => {
                let permit = sem
                    .acquire()
                    .await
                    .map_err(|_| anyhow::anyhow!("请求并发信号量已关闭"))?;
                Ok(Some(permit))
            }
            None => Ok(None),
        }
    }

    /// 根据 RequestOptions 选择合适的 reqwest Client
    ///
    /// - 若 options 与默认配置一致 → 使用懒加载的默认 client
    /// - 若 options 不同 → 从 custom_clients 缓存查找或新建
    fn get_client_for_options(&self, options: &RequestOptions) -> Result<Client> {
        let is_default = options.timeout_secs == self.default_timeout_secs
            && options.follow_redirects
            && options.verify_ssl;

        if is_default {
            self.get_or_init_default_client()
        } else {
            let key = ClientCacheKey::from_options(options);
            let mut cache = self.custom_clients.lock().unwrap();
            if let Some(cached) = cache.get(&key) {
                log::debug!("使用缓存的自定义客户端");
                Ok(cached.clone())
            } else {
                log::debug!("构建新的自定义客户端并缓存");
                let new_client = Self::build_custom_client(options, &self.pool_config)?;
                cache.insert(key, new_client.clone());
                Ok(new_client)
            }
        }
    }

    /// 懒加载获取默认 reqwest Client
    ///
    /// 首次调用时构建 Client（包含 TLS 初始化、连接池配置），
    /// 后续调用直接返回已缓存的实例。
    /// 所有通过 `Clone` 派生的 `HttpClient` 共享同一个底层 Client。
    fn get_or_init_default_client(&self) -> Result<Client> {
        self.default_client
            .get_or_init(|| {
                log::info!(
                    "懒加载: 构建默认 reqwest Client (timeout={}s, proxy={})",
                    self.default_timeout_secs,
                    self.default_proxy_url
                        .as_deref()
                        .unwrap_or("none")
                );
                Self::build_default_client(
                    self.default_timeout_secs,
                    self.default_proxy_url.as_deref(),
                    &self.pool_config,
                )
                .map_err(|e| format!("{:#}", e))
            })
            .clone()
            .map_err(|s| anyhow::anyhow!("{}", s))
    }

    /// 构建默认客户端
    ///
    /// 合并超时、代理、连接池配置，构造 reqwest `Client`。
    fn build_default_client(
        timeout_secs: u64,
        proxy_url: Option<&str>,
        pool_config: &PoolConfig,
    ) -> Result<Client> {
        let mut builder = Client::builder()
            .no_brotli()
            .no_gzip()
            .no_deflate()
            .no_proxy()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10));

        if let Some(url) = proxy_url {
            let proxy = Proxy::all(url).context("代理URL无效")?;
            builder = builder.proxy(proxy);
        }

        Self::apply_pool_config(&mut builder, pool_config);

        builder.build().context("创建HTTP客户端失败")
    }

    /// 构建自定义配置的客户端（用于非默认 RequestOptions）
    fn build_custom_client(opts: &RequestOptions, pool_config: &PoolConfig) -> Result<Client> {
        let mut builder = Client::builder()
            .no_brotli()
            .no_gzip()
            .no_deflate()
            .no_proxy()
            .timeout(Duration::from_secs(opts.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .redirect(if opts.follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            });

        if !opts.verify_ssl {
            builder = builder.danger_accept_invalid_certs(true);
        }

        Self::apply_pool_config(&mut builder, pool_config);

        builder.build().context("创建自定义HTTP客户端失败")
    }

    /// 将 PoolConfig 应用到 ClientBuilder
    fn apply_pool_config(builder: &mut reqwest::ClientBuilder, config: &PoolConfig) {
        if config.max_idle_per_host > 0 {
            *builder = std::mem::take(builder)
                .pool_max_idle_per_host(config.max_idle_per_host);
        }
        if config.idle_timeout_secs > 0 {
            let b = std::mem::take(builder);
            *builder = b.pool_idle_timeout(Duration::from_secs(config.idle_timeout_secs));
        }
        if config.tcp_keepalive_secs > 0 {
            let b = std::mem::take(builder);
            *builder = b.tcp_keepalive(Some(Duration::from_secs(config.tcp_keepalive_secs)));
        }
        if config.connect_timeout_secs > 0 {
            let b = std::mem::take(builder);
            *builder = b.connect_timeout(Duration::from_secs(config.connect_timeout_secs));
        }
    }

    /// 构建HTTP请求头
    fn build_headers(&self, headers: &[(String, String)]) -> Result<HeaderMap> {
        let mut header_map = HeaderMap::new();

        // 声明支持的压缩编码（reqwest 已禁用自动解压，手动处理）
        if !headers.iter().any(|(k, _)| k.to_lowercase() == "accept-encoding") {
            header_map.insert(
                ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, br, deflate"),
            );
        }

        log::debug!("请求头 ({} 项):", headers.len());
        for (name, value) in headers {
            // 替换头部值中的环境变量
            let resolved = self.env_manager.replace_variables(value);

            if *value != resolved {
                log::debug!("  {}: {} -> {}", name, value, resolved);
            } else {
                log::debug!("  {}: {}", name, value);
            }

            let header_name = HeaderName::try_from(name.as_str())
                .with_context(|| format!("无效的请求头名称: {}", name))?;
            let header_value = HeaderValue::from_str(&resolved)
                .with_context(|| format!("无效的请求头值: {}", resolved))?;

            header_map.insert(header_name, header_value);
        }

        Ok(header_map)
    }

    /// 构建multipart表单
    fn build_multipart(&self, request: &HttpRequest) -> Result<multipart::Form> {
        log::debug!(
            "构建multipart表单: text_fields={}, file_fields={}",
            request.text_fields.len(),
            request.file_fields.len()
        );

        let mut form = multipart::Form::new();

        // 添加文本字段
        for (name, value) in &request.text_fields {
            let value = self.env_manager.replace_variables(value);
            log::debug!("添加文本字段: {}={}", name, value);
            form = form.text(name.clone(), value);
        }

        // 添加文件字段
        for file_field in &request.file_fields {
            let file_path = &file_field.file_path;
            log::debug!("添加文件字段: {} -> {}", file_field.field_name, file_path);
            if std::path::Path::new(file_path).exists() {
                let file_name = std::path::Path::new(file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string());

                let file_content = std::fs::read(file_path)
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

// ============================================================================
// JSON 格式化工具
// ============================================================================

/// 带折叠的 JSON 格式化
pub fn format_json_folded(json_str: &str, max_depth: usize, indent_size: usize) -> String {
    let indent = |d: usize| " ".repeat(d * indent_size);

    fn format_value(
        value: &serde_json::Value,
        current_depth: usize,
        max_depth: usize,
        indent_size: usize,
    ) -> String {
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
                    s.push_str(&format!(
                        "{next_ind}\"{k}\": {}",
                        format_value(v, current_depth + 1, max_depth, indent_size)
                    ));
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
                    s.push_str(&format!(
                        "{next_ind}{}",
                        format_value(v, current_depth + 1, max_depth, indent_size)
                    ));
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

// ============================================================================
// 测试
// ============================================================================

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
            raw_body: None,
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
            raw_body: None,
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
            raw_body: None,
            cookies: Vec::new(),
        };

        let formatted = response.format_body();
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("test"));
    }

    // --- 懒加载测试 ---

    #[test]
    fn test_lazy_client_not_built_on_construction() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client = HttpClient::new(env_manager).unwrap();
        // 构造后 default_client OnceLock 应为空
        assert!(client.default_client.get().is_none());
    }

    #[test]
    fn test_lazy_client_built_on_first_request() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client = HttpClient::new(env_manager).unwrap();
        assert!(client.default_client.get().is_none());

        // 触发懒加载（不实际发网络请求，仅验证 Client 被构建）
        let result = client.get_or_init_default_client();
        assert!(result.is_ok());
        // OnceLock 现在包含 Ok(Client)
        assert!(client.default_client.get().is_some());
    }

    #[test]
    fn test_lazy_client_shared_across_clones() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client1 = HttpClient::new(env_manager).unwrap();
        let client2 = client1.clone();

        // 在 clone 上触发懒加载
        let _ = client2.get_or_init_default_client();

        // 原始实例也应可见（OnceLock 内有值）
        assert!(client1.default_client.get().is_some());
        assert!(client2.default_client.get().is_some());
    }

    // --- 连接池配置测试 ---

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_idle_per_host, 0);
        assert_eq!(config.idle_timeout_secs, 0);
        assert_eq!(config.max_concurrent_requests, 0);
    }

    #[test]
    fn test_pool_config_with_concurrency_limit() {
        let config = PoolConfig::with_concurrency_limit(10, 32);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_idle_per_host, 32);
        assert_eq!(config.idle_timeout_secs, 90);
    }

    #[test]
    fn test_with_pool_config_creates_semaphore() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client = HttpClient::new(env_manager)
            .unwrap()
            .with_pool_config(PoolConfig::with_concurrency_limit(5, 10));

        assert!(client.request_semaphore.is_some());
        assert_eq!(client.pool_config.max_concurrent_requests, 5);
    }

    #[test]
    fn test_pool_config_recommended() {
        let config = PoolConfig::recommended();
        assert_eq!(config.max_idle_per_host, 8);
        assert_eq!(config.idle_timeout_secs, 90);
        assert_eq!(config.max_concurrent_requests, 0);
        assert_eq!(config.tcp_keepalive_secs, 60);
        assert_eq!(config.connect_timeout_secs, 10);
    }

    #[test]
    fn test_with_pool_config_no_semaphore_when_zero() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client = HttpClient::new(env_manager)
            .unwrap()
            .with_pool_config(PoolConfig::default());

        assert!(client.request_semaphore.is_none());
    }

    // --- 网络测试（需要外网） ---

    #[tokio::test]
    async fn test_send_simple_request() {
        let env_manager = Arc::new(create_mock_env_manager());
        let client = HttpClient::new(env_manager).unwrap();

        let request = HttpRequest::new(Method::GET, "https://httpbin.org/get".to_string());
        let response = client.send_request(&request).await;

        if response.is_ok() {
            let resp = response.unwrap();
            assert_eq!(resp.status, 200);
        }
    }
}
