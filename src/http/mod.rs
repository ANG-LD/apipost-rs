//! HTTP模块
//!
//! 包含HTTP客户端和cURL解析器

mod client;
mod curl_parser;

pub use client::{HttpClient, HttpRequest, HttpResponse, RequestOptions};
pub use curl_parser::{generate_curl, generate_code, parse_curl};
