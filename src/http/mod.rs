//! HTTP模块
//!
//! 包含HTTP客户端和cURL解析器

mod client;
mod curl_parser;

pub use client::{
    format_json_folded, Cookie, FileField, HttpClient, HttpRequest, HttpResponse, PoolConfig,
    RequestOptions,
};
pub use curl_parser::{generate_code, generate_curl, parse_curl};
