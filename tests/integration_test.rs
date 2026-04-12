//! 集成测试
//!
//! 测试HTTP请求发送、环境变量替换、cURL解析等核心功能

use apipost_rs::app::{EnvironmentManager, HistoryManager};
use apipost_rs::http::{generate_code, generate_curl, parse_curl, HttpRequest, HttpResponse};
use apipost_rs::config::AppConfig;
use chrono::Utc;
use reqwest::Method;
use std::collections::HashMap;

/// 测试HTTP请求构建
#[test]
fn test_build_get_request() {
    let request = HttpRequest::new(Method::GET, "https://api.example.com/users".to_string());

    assert_eq!(request.method, Method::GET);
    assert_eq!(request.url, "https://api.example.com/users");
    assert!(request.headers.is_empty());
    assert!(request.body.is_none());
}

/// 测试带请求头的POST请求
#[test]
fn test_build_post_request_with_headers() {
    let request = HttpRequest::new(Method::POST, "https://api.example.com/users".to_string())
        .with_header("Content-Type".to_string(), "application/json".to_string())
        .with_header("Authorization".to_string(), "Bearer token123".to_string())
        .with_body(r#"{"name":"test"}"#.to_string());

    assert_eq!(request.method, Method::POST);
    assert_eq!(request.headers.len(), 2);
    assert!(request.body.is_some());
}

/// 测试cURL命令解析
#[test]
fn test_parse_curl_get() {
    let curl = "curl https://api.example.com/users";
    let request = parse_curl(curl).unwrap();

    assert_eq!(request.method, Method::GET);
    assert_eq!(request.url, "https://api.example.com/users");
}

/// 测试cURL命令解析带头部
#[test]
fn test_parse_curl_with_headers() {
    let curl = "curl -H 'Content-Type: application/json' -H 'Authorization: Bearer token' https://api.example.com/users";
    let request = parse_curl(curl).unwrap();

    assert_eq!(request.method, Method::GET);
    assert_eq!(request.headers.len(), 2);
    assert_eq!(request.headers[0].0, "Content-Type");
}

/// 测试cURL命令解析POST请求
#[test]
fn test_parse_curl_post() {
    let curl = "curl -X POST -d '{\"name\":\"test\"}' https://api.example.com/users";
    let request = parse_curl(curl).unwrap();

    assert_eq!(request.method, Method::POST);
    assert!(request.body.is_some());
}

/// 测试生成cURL命令
#[test]
fn test_generate_curl() {
    let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string())
        .with_header("Accept".to_string(), "application/json".to_string());

    let curl = generate_curl(&request);
    assert!(curl.contains("curl"));
    assert!(curl.contains("https://api.example.com"));
    assert!(curl.contains("-H"));
}

/// 测试生成Python代码
#[test]
fn test_generate_python_code() {
    let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
    let code = generate_code(&request, "python");

    assert!(code.contains("requests"));
    assert!(code.contains("https://api.example.com"));
}

/// 测试生成JavaScript代码
#[test]
fn test_generate_javascript_code() {
    let request = HttpRequest::new(Method::POST, "https://api.example.com".to_string())
        .with_body(r#"{"name":"test"}"#.to_string());

    let code = generate_code(&request, "javascript");
    assert!(code.contains("fetch"));
    assert!(code.contains("POST"));
}

/// 测试生成Go代码
#[test]
fn test_generate_go_code() {
    let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
    let code = generate_code(&request, "go");

    assert!(code.contains("http.NewRequest"));
    assert!(code.contains("https://api.example.com"));
}

/// 测试生成Rust代码
#[test]
fn test_generate_rust_code() {
    let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
    let code = generate_code(&request, "rust");

    assert!(code.contains("reqwest"));
    assert!(code.contains("https://api.example.com"));
}

/// 测试环境变量替换
#[test]
fn test_env_variable_replacement() {
    let manager = EnvironmentManager::new();
    manager.set_global("base_url".to_string(), "https://api.example.com".to_string());
    manager.set_current("api_key".to_string(), "test_key_123".to_string());

    let input = "{{base_url}}/users?api_key={{api_key}}";
    let result = manager.replace_variables(input);

    assert_eq!(result, "https://api.example.com/users?api_key=test_key_123");
}

/// 测试环境变量优先级
#[test]
fn test_env_variable_priority() {
    let manager = EnvironmentManager::new();
    manager.set_global("test_var".to_string(), "global_value".to_string());
    manager.set_current("test_var".to_string(), "current_value".to_string());

    let result = manager.replace_variables("{{test_var}}");
    assert_eq!(result, "current_value"); // 当前环境优先
}

/// 测试环境变量包含未知变量
#[test]
fn test_env_variable_unknown() {
    let manager = EnvironmentManager::new();
    manager.set_current("known".to_string(), "value".to_string());

    let result = manager.replace_variables("{{unknown}}");
    assert_eq!(result, "{{unknown}}"); // 未知变量保持原样
}

/// 测试提取变量名
#[test]
fn test_extract_variables() {
    let manager = EnvironmentManager::new();
    let vars = manager.extract_variables("{{base_url}}/{{path}}/{{id}}");

    assert_eq!(vars.len(), 3);
    assert!(vars.contains(&"base_url".to_string()));
    assert!(vars.contains(&"path".to_string()));
    assert!(vars.contains(&"id".to_string()));
}

/// 测试JSON导入导出
#[test]
fn test_env_json_import_export() {
    let manager = EnvironmentManager::new();
    manager.set_current("key1".to_string(), "value1".to_string());
    manager.set_current("key2".to_string(), "value2".to_string());

    let json = manager.export_to_json().unwrap();
    assert!(json.contains("key1"));
    assert!(json.contains("value1"));

    let manager2 = EnvironmentManager::new();
    manager2.load_from_json(&json).unwrap();

    assert_eq!(manager2.get_current("key1"), Some("value1".to_string()));
    assert_eq!(manager2.get_current("key2"), Some("value2".to_string()));
}

/// 测试HTTP响应状态判断
#[test]
fn test_response_status() {
    let success_response = HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: String::new(),
        time_ms: 100,
        size_bytes: 0,
    };
    assert!(success_response.is_success());
    assert_eq!(success_response.status_text(), "OK");

    let error_response = HttpResponse {
        status: 404,
        headers: HashMap::new(),
        body: String::new(),
        time_ms: 50,
        size_bytes: 0,
    };
    assert!(!error_response.is_success());
    assert_eq!(error_response.status_text(), "Not Found");
}

/// 测试响应格式化
#[test]
fn test_response_format_body() {
    let response = HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: r#"{"name":"test","value":123}"#.to_string(),
        time_ms: 100,
        size_bytes: 0,
    };

    let formatted = response.format_body();
    assert!(formatted.contains('\n')); // 应该被格式化
}

/// 测试响应格式化非JSON
#[test]
fn test_response_format_non_json() {
    let response = HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: "plain text response".to_string(),
        time_ms: 100,
        size_bytes: 0,
    };

    let formatted = response.format_body();
    assert_eq!(formatted, "plain text response"); // 非JSON保持原样
}

/// 测试配置默认值
#[test]
fn test_config_defaults() {
    let config = AppConfig::default();

    assert_eq!(config.general.language, "zh-CN");
    assert_eq!(config.general.theme, "dark");
    assert_eq!(config.general.timeout, 30);
    assert!(config.general.auto_save);
    assert!(!config.proxy.enabled);
}

/// 测试历史记录过滤
#[test]
fn test_history_filter_by_method() {
    let entries = vec![
        create_test_history("1", "GET", "https://api.example.com/1", 200),
        create_test_history("2", "POST", "https://api.example.com/2", 201),
        create_test_history("3", "GET", "https://api.example.com/3", 200),
    ];

    let get_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.method == "GET")
        .collect();

    assert_eq!(get_entries.len(), 2);
}

/// 创建测试用历史记录
fn create_test_history(id: &str, method: &str, url: &str, status: i32) -> apipost_rs::app::HistoryEntry {
    use uuid::Uuid;
    apipost_rs::app::HistoryEntry {
        id: id.to_string(),
        method: method.to_string(),
        url: url.to_string(),
        headers: None,
        body: None,
        response_status: Some(status),
        response_headers: None,
        response_body: None,
        response_time_ms: Some(100),
        created_at: Utc::now(),
    }
}
