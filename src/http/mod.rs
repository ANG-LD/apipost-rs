//! HTTP模块
//!
//! 包含HTTP客户端和cURL解析器

mod client;
mod curl_parser;

pub use client::{Cookie, HttpClient, HttpRequest, HttpResponse, RequestOptions, FileField, format_json_folded};
pub use curl_parser::{generate_curl, generate_code, parse_curl};
