//! cURL命令解析器
//!
//! 解析cURL命令字符串，提取HTTP请求的各个组成部分
//! 支持复杂的cURL命令，包括多种头部、请求体、认证等

use crate::http::client::HttpRequest;
use anyhow::Result;
use reqwest::Method;
use regex::Regex;

/// cURL命令解析器
pub struct CurlParser {
    /// 原始命令字符串
    input: String,
    /// 解析位置
    pos: usize,
}

impl CurlParser {
    /// 创建新的解析器
    pub fn new(input: String) -> Self {
        // 处理 shell 换行续行符（\\\n 或 \\\r\n）
        let input = input.replace("\\\r\n", " ").replace("\\\n", " ");
        // 处理 Windows CMD 续行符 ^\n 和 ^\r\n
        let input = input.replace("^\r\n", " ").replace("^\n", " ");
        // 处理 Windows CMD 转义符 ^X → X（非换行的任意字符）
        let mut processed = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '^' && i + 1 < chars.len() {
                processed.push(chars[i + 1]);
                i += 2;
            } else {
                processed.push(chars[i]);
                i += 1;
            }
        }
        Self { input: processed, pos: 0 }
    }

    /// 解析cURL命令
    pub fn parse(&mut self) -> Result<HttpRequest> {
        // 跳过初始的 "curl" 命令
        self.skip_whitespace();
        if self.input[self.pos..].starts_with("curl") {
            self.pos += 4;
        }

        self.skip_whitespace();

        let mut method = Method::GET;
        let mut url = String::new();
        let mut headers: Vec<(String, String)> = Vec::new();
        let mut body: Option<String> = None;
        let mut user_agent: Option<String> = None;
        let mut auth: Option<(String, String)> = None;

        while self.pos < self.input.len() {
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                break;
            }

            let ch = self.current_char();

            if ch == '-' {
                // 解析选项
                let option = self.parse_option()?;

                match option.name.as_str() {
                    "X" | "request" => {
                        method = Method::try_from(
                            option.value.as_deref().unwrap_or("GET").to_uppercase().as_str(),
                        )
                        .unwrap_or(Method::GET);
                    }
                    "H" | "header" => {
                        if let Some(value) = option.value {
                            if let Some((k, v)) = value.split_once(':') {
                                headers.push((k.trim().to_string(), v.trim().to_string()));
                            }
                        }
                    }
                    "d" | "data" | "data-raw" => {
                        method = Method::POST;
                        body = Some(option.value.unwrap_or_default());
                    }
                    "u" | "user" => {
                        if let Some(value) = option.value {
                            if let Some((username, password)) = value.split_once(':') {
                                auth = Some((username.to_string(), password.to_string()));
                            }
                        }
                    }
                    "A" | "user-agent" => {
                        user_agent = option.value;
                    }
                    "L" | "location" => {}
                    "k" | "insecure" => {}
                    "s" | "silent" | "S" | "show-error" => {}
                    _ => {
                        if let Some(value) = option.value {
                            if value.starts_with("http://") || value.starts_with("https://") {
                                url = value;
                            }
                        }
                    }
                }
            } else if ch == '\'' || ch == '"' {
                let quote = ch;
                let s = self.parse_quoted_string(quote)?;
                if url.is_empty() && (s.starts_with("http://") || s.starts_with("https://")) {
                    url = s;
                }
            } else if ch.is_whitespace() {
                self.pos += 1;
            } else {
                let word = self.parse_word();
                if !word.is_empty()
                    && (word.starts_with("http://") || word.starts_with("https://"))
                {
                    url = word;
                }
            }
        }

        // 如果没有找到URL，尝试从参数中提取
        if url.is_empty() {
            let url_re = Regex::new(r#"https?://[^\s'"]+"#).unwrap();
            if let Some(m) = url_re.find(&self.input) {
                let found_url = m.as_str();
                let mut cleaned = found_url.to_string();
                while cleaned.ends_with('\'') || cleaned.ends_with('"') {
                    cleaned.pop();
                }
                url = cleaned;
            }
        }

        if url.is_empty() {
            anyhow::bail!("无法从cURL命令中提取URL");
        }

        // 构建HTTP请求
        let mut request = HttpRequest::new(method, url);

        for (name, value) in headers {
            request = request.with_header(name, value);
        }

        // 添加认证头
        if let Some((username, password)) = auth {
            let encoded = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", username, password),
            );
            request = request.with_header(
                "Authorization".to_string(),
                format!("Basic {}", encoded),
            );
        }

        // 添加用户代理
        if let Some(ua) = user_agent {
            request = request.with_header("User-Agent".to_string(), ua);
        }

        // 设置请求体
        if let Some(b) = body {
            request = request.with_body(b);
        }

        Ok(request)
    }

    /// 解析选项
    fn parse_option(&mut self) -> Result<CurlOption> {
        self.pos += 1;

        if self.current_char() == '-' {
            self.pos += 1;
            let name = self.parse_word();
            self.skip_whitespace();

            let value = if self.current_char() == '=' {
                self.pos += 1;
                Some(self.parse_value())
            } else if self.pos >= self.input.len() {
                None
            } else {
                let ch = self.current_char();
                if ch == '-' {
                    // 下一个是另一个选项
                    None
                } else {
                    let val = self.parse_value();
                    if val.is_empty() { None } else { Some(val) }
                }
            };

            Ok(CurlOption { name, value })
        } else {
            let name = self.input[self.pos..].chars().next().unwrap().to_string();
            self.pos += 1;
            self.skip_whitespace();

            let value = if self.pos < self.input.len() {
                let ch = self.current_char();
                if ch == '=' {
                    self.pos += 1;
                    Some(self.parse_value())
                } else if ch == '\'' || ch == '"' {
                    Some(self.parse_value())
                } else if ch == ' ' || ch == '\t' {
                    let peek = self.peek_char();
                    if peek == '-' {
                        None
                    } else {
                        Some(self.parse_value())
                    }
                } else if ch != '-' && !ch.is_whitespace() {
                    let val = self.parse_value();
                    if val.starts_with("http") || val.starts_with("-") || val.is_empty() {
                        if val.is_empty() { None } else { Some(val) }
                    } else {
                        Some(val)
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Ok(CurlOption { name, value })
        }
    }

    /// 解析带引号的字符串
    fn parse_quoted_string(&mut self, quote: char) -> Result<String> {
        self.pos += 1;
        let mut result = String::new();

        while self.pos < self.input.len() && self.current_char() != quote {
            let ch = self.current_char();
            if ch == '\\' && self.pos + 1 < self.input.len() {
                self.pos += 1;
                result.push(self.current_char());
            } else {
                result.push(ch);
            }
            self.pos += 1;
        }

        if self.pos < self.input.len() {
            self.pos += 1;
        }

        Ok(result)
    }

    /// 解析普通单词
    fn parse_word(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_whitespace() || ch == '\'' || ch == '"' || ch == '=' {
                break;
            }
            self.pos += 1;
        }
        self.input[start..self.pos].to_string()
    }

    /// 解析选项值
    fn parse_value(&mut self) -> String {
        let mut value = String::new();

        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch == '\'' || ch == '"' {
                if let Ok(s) = self.parse_quoted_string(ch) {
                    value.push_str(&s);
                }
                break;
            } else if ch.is_whitespace() {
                break;
            } else if ch == '\\' && self.pos + 1 < self.input.len() {
                self.pos += 1;
                value.push(self.current_char());
            } else {
                value.push(ch);
            }
            self.pos += 1;
        }

        value
    }

    /// 跳过空白字符
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.pos += 1;
        }
    }

    /// 获取当前位置的字符
    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    /// 查看下一个字符
    fn peek_char(&self) -> char {
        let next_pos = self.pos + 1;
        if next_pos < self.input.len() {
            self.input[next_pos..].chars().next().unwrap_or('\0')
        } else {
            '\0'
        }
    }
}

/// cURL选项结构
#[derive(Debug)]
struct CurlOption {
    name: String,
    value: Option<String>,
}

/// 从cURL命令字符串解析为HTTP请求
pub fn parse_curl(curl_command: &str) -> Result<HttpRequest> {
    let mut parser = CurlParser::new(curl_command.to_string());
    parser.parse()
}

/// 生成cURL命令
pub fn generate_curl(request: &HttpRequest) -> String {
    let mut cmd = String::from("curl");

    if request.method != "GET" {
        cmd.push_str(&format!(" -X {}", request.method));
    }

    for (name, value) in &request.headers {
        cmd.push_str(&format!(" -H '{}: {}'", name, value));
    }

    if let Some(body) = &request.body {
        cmd.push_str(&format!(" -d '{}'", body.replace('\'', "'\\''")));
    }

    cmd.push_str(&format!(" '{}'", request.url));

    cmd
}

/// 生成多语言代码
pub fn generate_code(request: &HttpRequest, language: &str) -> String {
    match language.to_lowercase().as_str() {
        "python" | "py" => generate_python(request),
        "javascript" | "js" => generate_javascript(request),
        "go" => generate_go(request),
        "rust" | "rs" => generate_rust(request),
        "java" => generate_java(request),
        "php" => generate_php(request),
        "curl" => generate_curl(request),
        _ => generate_curl(request),
    }
}

/// 生成Python代码
fn generate_python(request: &HttpRequest) -> String {
    let method_lower = request.method.to_lowercase();
    let url = &request.url;
    let headers: Vec<String> = request
        .headers
        .iter()
        .map(|(k, v)| format!("    '{}': '{}'", k, v))
        .collect();
    let has_body = request.body.is_some();
    let body_code = request
        .body
        .as_ref()
        .map(|b| format!("    'data': {}", serde_json::to_string(b).unwrap_or_else(|_| b.clone())))
        .unwrap_or_default();

    let headers_str = headers.join(",\n");
    let body_str = if has_body { &body_code } else { "{}" };
    let data_val = if has_body { "data" } else { "None" };
    let extra_arg = if has_body { ",\n    data=data" } else { "" };

    format!(
        "import requests\n\nurl = '{}'\nheaders = {{\n{}\n{}}}\ndata = {}\n\nresponse = requests.{}(\n    url,\n    headers=headers{}\n)\nprint(response.status_code)\nprint(response.text)\n",
        url, headers_str, body_str, data_val, method_lower, extra_arg
    )
}

/// 生成JavaScript代码
fn generate_javascript(request: &HttpRequest) -> String {
    let url = &request.url;
    let headers: Vec<String> = request
        .headers
        .iter()
        .map(|(k, v)| format!("    '{}': '{}'", k, v))
        .collect();
    let has_body = request.body.is_some();
    let body_code = request
        .body
        .as_ref()
        .map(|b| format!("    body: JSON.stringify({})", b))
        .unwrap_or_default();

    let headers_str = headers.join(",\n");
    let body_part = if has_body { format!("  {},\n", body_code) } else { String::new() };

    format!(
        "const options = {{\n  method: '{}',\n  headers: {{\n{}\n  }},\n{}}};\n\nfetch('{}', options)\n  .then(response => response.json())\n  .then(data => console.log(data))\n  .catch(error => console.error('Error:', error));\n",
        request.method, url, headers_str, body_part
    )
}

/// 生成Go代码
fn generate_go(request: &HttpRequest) -> String {
    let url = &request.url;
    let headers: Vec<String> = request
        .headers
        .iter()
        .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
        .collect();
    let body_import = if request.body.is_some() { "\n    \"strings\"" } else { "" };
    let body_decl = request.body.as_ref().map(|b| {
        let b_str = serde_json::to_string(b).unwrap_or_else(|_| format!("\"{}\"", b));
        format!("    var body = strings.NewReader({})\n", b_str)
    }).unwrap_or_default();
    let body_arg = if request.body.is_some() { "body,\n        " } else { "" };
    let headers_code = if headers.is_empty() {
        String::new()
    } else {
        headers.iter()
            .map(|h| format!("    req.Header.Set({})", h))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "package main\n\nimport (\n    \"fmt\"\n    \"net/http\"{}\n)\n\nfunc main() {{\n{}\n{}\n    req, err := http.NewRequest(\"{}\", \"{}\", {})\n    if err != nil {{\n        panic(err)\n    }}\n\n{}\n    resp, err := http.DefaultClient.Do(req)\n    if err != nil {{\n        panic(err)\n    }}\n    defer resp.Body.Close()\n\n    fmt.Println(\"Response status:\", resp.Status)\n}}\n",
        body_import, url, body_decl, request.method, url, body_arg, headers_code
    )
}

/// 生成Rust代码
fn generate_rust(request: &HttpRequest) -> String {
    let method_lower = request.method.to_lowercase();
    let url = &request.url;
    let headers: Vec<String> = request
        .headers
        .iter()
        .map(|(k, v)| format!("    .header(\"{}\", \"{}\")", k, v))
        .collect();
    let body_code = request
        .body
        .as_ref()
        .map(|b| {
            let b_str = serde_json::to_string(b).unwrap_or_else(|_| format!("\"{}\"", b));
            format!("\n    .body({})", b_str)
        })
        .unwrap_or_default();
    let headers_str = headers.join("\n");

    format!(
        "use reqwest::Client;\n\n#[tokio::main]\nasync fn main() -> Result<(), reqwest::Error> {{\n    let client = Client::new();\n\n    let response = client.{}()\n        (\"{}\"\n    ){}\n{}\n        .await?;\n\n    println!(\"Status: {{}}\", response.status());\n    println!(\"Body: {{}}\", response.text().await?);\n    Ok(())\n}}\n",
        method_lower, url, headers_str, body_code
    )
}

/// 生成Java代码
#[allow(unused_variables)]
fn generate_java(request: &HttpRequest) -> String {
    let url = &request.url;
    let headers_code: String = request
        .headers
        .iter()
        .map(|(k, v)| format!("        request.addHeader(\"{}\", \"{}\");", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    let body_code = request.body.as_ref()
        .map(|b| {
            format!(
                "        StringEntity entity = new StringEntity(\"{}\", \"UTF-8\");\n        request.setEntity(entity);",
                b.replace('"', "\\\"")
            )
        })
        .unwrap_or_default();

    format!(
        "import org.apache.http.client.methods.CloseableHttpResponse;\nimport org.apache.http.client.methods.Http{};\nimport org.apache.http.impl.client.CloseableHttpClient;\nimport org.apache.http.impl.client.HttpClients;\nimport org.apache.http.util.EntityUtils;\n\npublic class ApiRequest {{\n    public static void main(String[] args) throws Exception {{\n        CloseableHttpClient client = HttpClients.createDefault();\n        Http{{}} request = new Http{{}}(\"{}\");\n{}\n{}\n        try (CloseableHttpResponse response = client.execute(request)) {{\n            System.out.println(\"Status: \" + response.getStatusLine().getStatusCode());\n            System.out.println(\"Body: \" + EntityUtils.toString(response.getEntity()));\n        }}\n    }}\n}}\n",
        request.method, url, headers_code, body_code
    )
}

/// 生成PHP代码
fn generate_php(request: &HttpRequest) -> String {
    let url = &request.url;
    let headers: Vec<String> = request
        .headers
        .iter()
        .map(|(k, v)| format!("    \"{}: {}\"", k, v))
        .collect();
    let has_body = request.body.is_some();
    let body_code = request
        .body
        .as_ref()
        .map(|b| format!("    CURLOPT_POSTFIELDS => '{}',", b.replace('\'', "\\'")))
        .unwrap_or_default();

    let headers_str = headers.join(",\n");
    let body_part = if has_body { format!("\n{}", body_code) } else { String::new() };

    format!(
        "<?php\n\n$curl = curl_init();\n\ncurl_setopt_array($curl, [\n    CURLOPT_URL => \"{}\",\n    CURLOPT_RETURNTRANSFER => true,\n    CURLOPT_ENCODING => \"\",\n    CURLOPT_MAXREDIRS => 10,\n    CURLOPT_TIMEOUT => 30,\n    CURLOPT_HTTP_VERSION => CURL_HTTP_VERSION_1_1,\n    CURLOPT_CUSTOMREQUEST => \"{}\",\n    CURLOPT_HTTPHEADER => [\n{}\n    ],{}\n]);\n\n$response = curl_exec($curl);\n$err = curl_error($curl);\n\ncurl_close($curl);\n\nif ($err) {{\n    echo \"Error: \" . $err;\n}} else {{\n    echo $response;\n}}\n",
        url, request.method, headers_str, body_part
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_curl() {
        let curl = "curl https://api.example.com/users";
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "https://api.example.com/users");
    }

    #[test]
    fn test_parse_curl_with_headers() {
        let curl = "curl -H 'Content-Type: application/json' -H 'Authorization: Bearer token123' https://api.example.com/users";
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers[0].0, "Content-Type");
        assert_eq!(request.headers[0].1, "application/json");
    }

    #[test]
    fn test_parse_curl_post_with_data() {
        let curl = "curl -X POST -d '{\"name\":\"test\"}' https://api.example.com/users";
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.method, "POST");
        assert!(request.body.is_some());
    }

    #[test]
    fn test_parse_curl_with_user_agent() {
        let curl = "curl -A 'Mozilla/5.0' https://api.example.com";
        let request = parse_curl(curl).unwrap();
        let has_ua = request.headers.iter().any(|(k, _)| k == "User-Agent");
        assert!(has_ua);
    }

    #[test]
    fn test_generate_curl() {
        let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string())
            .with_header("Content-Type".to_string(), "application/json".to_string());
        let curl = generate_curl(&request);
        assert!(curl.contains("curl"));
        assert!(curl.contains("https://api.example.com"));
        assert!(curl.contains("-H"));
    }

    #[test]
    fn test_generate_python_code() {
        let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
        let code = generate_code(&request, "python");
        assert!(code.contains("import requests"));
        assert!(code.contains("https://api.example.com"));
    }

    #[test]
    fn test_generate_javascript_code() {
        let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
        let code = generate_code(&request, "javascript");
        assert!(code.contains("fetch"));
        assert!(code.contains("https://api.example.com"));
    }

    #[test]
    fn test_generate_rust_code() {
        let request = HttpRequest::new(Method::GET, "https://api.example.com".to_string());
        let code = generate_code(&request, "rust");
        assert!(code.contains("reqwest"));
        assert!(code.contains("https://api.example.com"));
    }

    #[test]
    fn test_parse_windows_cmd_curl() {
        let curl = "curl -X POST \"https://api.example.com/save\" ^
  -H \"Content-Type: application/json\" ^
  --data-raw ^\"{^\\^\"ids^\\^\":^[2^],^\\^\"name^\\^\":^\\^\"test^\\^\"}^\"";
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://api.example.com/save");
        assert!(request.body.is_some());
        assert_eq!(request.body.unwrap(), "{\"ids\":[2],\"name\":\"test\"}");
    }
}
