//! 国际化模块
//!
//! 支持中文和英文界面切换

use std::collections::HashMap;

/// 国际化管理器
pub struct I18nManager {
    /// 当前语言
    language: String,
    /// 翻译字典
    translations: HashMap<String, String>,
}

impl I18nManager {
    /// 创建新的国际化管理器
    pub fn new(language: &str) -> Self {
        let translations = match language {
            "en-US" | "en" => Self::english(),
            _ => Self::chinese(),
        };

        Self {
            language: language.to_string(),
            translations,
        }
    }

    /// 获取翻译文本
    pub fn get(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// 获取当前语言
    pub fn current_language(&self) -> &str {
        &self.language
    }

    /// 设置语言
    pub fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
        self.translations = match language {
            "en-US" | "en" => Self::english(),
            _ => Self::chinese(),
        };
    }

    /// 中文翻译字典
    fn chinese() -> HashMap<String, String> {
        let mut map = HashMap::new();

        // 通用
        map.insert("app.name".to_string(), "ApiPost-Rs".to_string());
        map.insert("app.title".to_string(), "API 测试工具".to_string());

        // 菜单和按钮
        map.insert("menu.file".to_string(), "文件".to_string());
        map.insert("menu.edit".to_string(), "编辑".to_string());
        map.insert("menu.view".to_string(), "视图".to_string());
        map.insert("menu.help".to_string(), "帮助".to_string());
        map.insert("button.send".to_string(), "发送".to_string());
        map.insert("button.cancel".to_string(), "取消".to_string());
        map.insert("button.save".to_string(), "保存".to_string());
        map.insert("button.delete".to_string(), "删除".to_string());
        map.insert("button.copy".to_string(), "复制".to_string());
        map.insert("button.clear".to_string(), "清空".to_string());
        map.insert("button.import".to_string(), "导入".to_string());
        map.insert("button.export".to_string(), "导出".to_string());

        // HTTP方法
        map.insert("method.get".to_string(), "GET".to_string());
        map.insert("method.post".to_string(), "POST".to_string());
        map.insert("method.put".to_string(), "PUT".to_string());
        map.insert("method.delete".to_string(), "DELETE".to_string());
        map.insert("method.patch".to_string(), "PATCH".to_string());
        map.insert("method.head".to_string(), "HEAD".to_string());
        map.insert("method.options".to_string(), "OPTIONS".to_string());

        // 请求构建器
        map.insert("request.url".to_string(), "请求URL".to_string());
        map.insert("request.url.placeholder".to_string(), "输入请求URL或粘贴cURL命令".to_string());
        map.insert("request.headers".to_string(), "请求头".to_string());
        map.insert("request.body".to_string(), "请求体".to_string());
        map.insert("request.params".to_string(), "参数".to_string());
        map.insert("request.auth".to_string(), "认证".to_string());

        // 响应
        map.insert("response.title".to_string(), "响应".to_string());
        map.insert("response.body".to_string(), "响应体".to_string());
        map.insert("response.headers".to_string(), "响应头".to_string());
        map.insert("response.status".to_string(), "状态码".to_string());
        map.insert("response.time".to_string(), "响应时间".to_string());
        map.insert("response.size".to_string(), "响应大小".to_string());
        map.insert("response.preview".to_string(), "预览".to_string());
        map.insert("response.raw".to_string(), "原始".to_string());
        map.insert("response.formatted".to_string(), "格式化".to_string());

        // 历史记录
        map.insert("history.title".to_string(), "历史记录".to_string());
        map.insert("history.search".to_string(), "搜索历史...".to_string());
        map.insert("history.clear".to_string(), "清空历史".to_string());
        map.insert("history.empty".to_string(), "暂无历史记录".to_string());
        map.insert("history.confirm_clear".to_string(), "确定要清空所有历史记录吗？".to_string());

        // 环境变量
        map.insert("env.title".to_string(), "环境变量".to_string());
        map.insert("env.global".to_string(), "全局变量".to_string());
        map.insert("env.current".to_string(), "当前环境".to_string());
        map.insert("env.add".to_string(), "添加变量".to_string());
        map.insert("env.name".to_string(), "变量名".to_string());
        map.insert("env.value".to_string(), "变量值".to_string());
        map.insert("env.no_env".to_string(), "未选择环境".to_string());

        // 代码生成
        map.insert("code.title".to_string(), "生成代码".to_string());
        map.insert("code.language".to_string(), "选择语言".to_string());
        map.insert("code.python".to_string(), "Python".to_string());
        map.insert("code.javascript".to_string(), "JavaScript".to_string());
        map.insert("code.go".to_string(), "Go".to_string());
        map.insert("code.rust".to_string(), "Rust".to_string());
        map.insert("code.java".to_string(), "Java".to_string());
        map.insert("code.php".to_string(), "PHP".to_string());
        map.insert("code.curl".to_string(), "cURL".to_string());
        map.insert("code.copied".to_string(), "代码已复制到剪贴板".to_string());

        // 主题
        map.insert("theme.title".to_string(), "主题".to_string());
        map.insert("theme.light".to_string(), "浅色".to_string());
        map.insert("theme.dark".to_string(), "深色".to_string());

        // 语言
        map.insert("language.title".to_string(), "语言".to_string());
        map.insert("language.zh".to_string(), "中文".to_string());
        map.insert("language.en".to_string(), "英文".to_string());

        // 状态消息
        map.insert("status.ready".to_string(), "就绪".to_string());
        map.insert("status.sending".to_string(), "发送中...".to_string());
        map.insert("status.success".to_string(), "请求成功".to_string());
        map.insert("status.error".to_string(), "请求失败".to_string());
        map.insert("status.loading".to_string(), "加载中...".to_string());

        // 错误消息
        map.insert("error.network".to_string(), "网络错误".to_string());
        map.insert("error.timeout".to_string(), "请求超时".to_string());
        map.insert("error.invalid_url".to_string(), "无效的URL".to_string());
        map.insert("error.parse_failed".to_string(), "解析失败".to_string());
        map.insert("error.unknown".to_string(), "未知错误".to_string());

        // 侧边栏
        map.insert("sidebar.collections".to_string(), "收藏".to_string());
        map.insert("sidebar.history".to_string(), "历史".to_string());
        map.insert("sidebar.env".to_string(), "环境".to_string());

        map
    }

    /// 英文翻译字典
    fn english() -> HashMap<String, String> {
        let mut map = HashMap::new();

        // General
        map.insert("app.name".to_string(), "ApiPost-Rs".to_string());
        map.insert("app.title".to_string(), "API Testing Tool".to_string());

        // Menu and Buttons
        map.insert("menu.file".to_string(), "File".to_string());
        map.insert("menu.edit".to_string(), "Edit".to_string());
        map.insert("menu.view".to_string(), "View".to_string());
        map.insert("menu.help".to_string(), "Help".to_string());
        map.insert("button.send".to_string(), "Send".to_string());
        map.insert("button.cancel".to_string(), "Cancel".to_string());
        map.insert("button.save".to_string(), "Save".to_string());
        map.insert("button.delete".to_string(), "Delete".to_string());
        map.insert("button.copy".to_string(), "Copy".to_string());
        map.insert("button.clear".to_string(), "Clear".to_string());
        map.insert("button.import".to_string(), "Import".to_string());
        map.insert("button.export".to_string(), "Export".to_string());

        // HTTP Methods
        map.insert("method.get".to_string(), "GET".to_string());
        map.insert("method.post".to_string(), "POST".to_string());
        map.insert("method.put".to_string(), "PUT".to_string());
        map.insert("method.delete".to_string(), "DELETE".to_string());
        map.insert("method.patch".to_string(), "PATCH".to_string());
        map.insert("method.head".to_string(), "HEAD".to_string());
        map.insert("method.options".to_string(), "OPTIONS".to_string());

        // Request Builder
        map.insert("request.url".to_string(), "Request URL".to_string());
        map.insert("request.url.placeholder".to_string(), "Enter request URL or paste cURL command".to_string());
        map.insert("request.headers".to_string(), "Headers".to_string());
        map.insert("request.body".to_string(), "Body".to_string());
        map.insert("request.params".to_string(), "Params".to_string());
        map.insert("request.auth".to_string(), "Auth".to_string());

        // Response
        map.insert("response.title".to_string(), "Response".to_string());
        map.insert("response.body".to_string(), "Body".to_string());
        map.insert("response.headers".to_string(), "Headers".to_string());
        map.insert("response.status".to_string(), "Status".to_string());
        map.insert("response.time".to_string(), "Time".to_string());
        map.insert("response.size".to_string(), "Size".to_string());
        map.insert("response.preview".to_string(), "Preview".to_string());
        map.insert("response.raw".to_string(), "Raw".to_string());
        map.insert("response.formatted".to_string(), "Formatted".to_string());

        // History
        map.insert("history.title".to_string(), "History".to_string());
        map.insert("history.search".to_string(), "Search history...".to_string());
        map.insert("history.clear".to_string(), "Clear History".to_string());
        map.insert("history.empty".to_string(), "No history".to_string());
        map.insert("history.confirm_clear".to_string(), "Are you sure you want to clear all history?".to_string());

        // Environment Variables
        map.insert("env.title".to_string(), "Environment".to_string());
        map.insert("env.global".to_string(), "Global".to_string());
        map.insert("env.current".to_string(), "Current".to_string());
        map.insert("env.add".to_string(), "Add Variable".to_string());
        map.insert("env.name".to_string(), "Variable Name".to_string());
        map.insert("env.value".to_string(), "Variable Value".to_string());
        map.insert("env.no_env".to_string(), "No environment selected".to_string());

        // Code Generation
        map.insert("code.title".to_string(), "Generate Code".to_string());
        map.insert("code.language".to_string(), "Select Language".to_string());
        map.insert("code.python".to_string(), "Python".to_string());
        map.insert("code.javascript".to_string(), "JavaScript".to_string());
        map.insert("code.go".to_string(), "Go".to_string());
        map.insert("code.rust".to_string(), "Rust".to_string());
        map.insert("code.java".to_string(), "Java".to_string());
        map.insert("code.php".to_string(), "PHP".to_string());
        map.insert("code.curl".to_string(), "cURL".to_string());
        map.insert("code.copied".to_string(), "Code copied to clipboard".to_string());

        // Theme
        map.insert("theme.title".to_string(), "Theme".to_string());
        map.insert("theme.light".to_string(), "Light".to_string());
        map.insert("theme.dark".to_string(), "Dark".to_string());

        // Language
        map.insert("language.title".to_string(), "Language".to_string());
        map.insert("language.zh".to_string(), "Chinese".to_string());
        map.insert("language.en".to_string(), "English".to_string());

        // Status Messages
        map.insert("status.ready".to_string(), "Ready".to_string());
        map.insert("status.sending".to_string(), "Sending...".to_string());
        map.insert("status.success".to_string(), "Request successful".to_string());
        map.insert("status.error".to_string(), "Request failed".to_string());
        map.insert("status.loading".to_string(), "Loading...".to_string());

        // Error Messages
        map.insert("error.network".to_string(), "Network error".to_string());
        map.insert("error.timeout".to_string(), "Request timeout".to_string());
        map.insert("error.invalid_url".to_string(), "Invalid URL".to_string());
        map.insert("error.parse_failed".to_string(), "Parse failed".to_string());
        map.insert("error.unknown".to_string(), "Unknown error".to_string());

        // Sidebar
        map.insert("sidebar.collections".to_string(), "Collections".to_string());
        map.insert("sidebar.history".to_string(), "History".to_string());
        map.insert("sidebar.env".to_string(), "Environment".to_string());

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chinese_translations() {
        let i18n = I18nManager::new("zh-CN");
        assert_eq!(i18n.get("button.send"), "发送");
        assert_eq!(i18n.get("request.url"), "请求URL");
    }

    #[test]
    fn test_english_translations() {
        let i18n = I18nManager::new("en-US");
        assert_eq!(i18n.get("button.send"), "Send");
        assert_eq!(i18n.get("request.url"), "Request URL");
    }

    #[test]
    fn test_unknown_key() {
        let i18n = I18nManager::new("en-US");
        assert_eq!(i18n.get("unknown.key"), "unknown.key");
    }

    #[test]
    fn test_switch_language() {
        let mut i18n = I18nManager::new("en-US");
        assert_eq!(i18n.get("button.send"), "Send");

        i18n.set_language("zh-CN");
        assert_eq!(i18n.get("button.send"), "发送");
    }
}
