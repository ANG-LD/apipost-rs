//! 国际化模块
//!
//! 支持中文和英文界面切换

use std::collections::HashMap;
use std::sync::Arc;

/// 翻译字典：值是 `Arc<str>`，查表后取出只需引用计数 +1，
/// 界面每帧要取上百条文案，用 String 的话每帧就是上百次堆分配。
pub type Translations = HashMap<String, Arc<str>>;

/// 国际化管理器
#[derive(Clone)]
pub struct I18nManager {
    /// 当前语言
    language: String,
    /// 翻译字典（Arc 共享，Clone 零分配）
    translations: Arc<Translations>,
}

impl I18nManager {
    /// 创建新的国际化管理器
    ///
    /// 字典本身还是用 `HashMap<String, String>` 的字面量写，只在构造时转一次
    /// `Arc<str>`，这样维护翻译表时不用改写法。
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            translations: Self::build(language),
        }
    }

    /// 获取翻译文本（零分配查询）
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.translations
            .get(key)
            .map(|s| &**s)
            .unwrap_or(key)
    }

    /// 获取翻译字典的 Arc（供外部缓存）
    pub fn translations_arc(&self) -> Arc<Translations> {
        Arc::clone(&self.translations)
    }

    /// 取文案并返回 `Arc<str>`：命中时是纯引用计数操作，不复制字符串。
    /// 界面渲染走这条路径；`get()` 继续给需要 `&str` 的地方用。
    pub fn get_shared(&self, key: &str) -> Arc<str> {
        match self.translations.get(key) {
            Some(value) => Arc::clone(value),
            // 只有 key 不存在（翻译漏了）才会走到这里，才会真的分配
            None => Arc::from(key),
        }
    }

    /// 获取当前语言
    pub fn current_language(&self) -> &str {
        &self.language
    }

    /// 设置语言
    pub fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
        self.translations = Self::build(language);
    }

    /// 按语言构建字典（字面量表 -> `Arc<str>`，构造时转一次）
    fn build(language: &str) -> Arc<Translations> {
        let raw = match language {
            "en-US" | "en" => Self::english(),
            _ => Self::chinese(),
        };
        Arc::new(
            raw.into_iter()
                .map(|(key, value)| (key, Arc::from(value)))
                .collect(),
        )
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
        map.insert("request.pre_request".to_string(), "预请求".to_string());
        map.insert("request.tests".to_string(), "测试".to_string());
        map.insert("request.settings".to_string(), "设置".to_string());
        map.insert("ui.key".to_string(), "键".to_string());
        map.insert("ui.value".to_string(), "值".to_string());
        map.insert("ui.send".to_string(), "发送".to_string());
        map.insert("ui.sending".to_string(), "发送中...".to_string());
        map.insert("ui.no_history".to_string(), "暂无历史记录".to_string());
        map.insert("ui.json".to_string(), "JSON".to_string());
        map.insert("ui.xml".to_string(), "XML".to_string());
        map.insert("ui.text".to_string(), "Text".to_string());
        map.insert("ui.html".to_string(), "HTML".to_string());
        map.insert("ui.pretty".to_string(), "格式化".to_string());
        map.insert("ui.raw".to_string(), "原始".to_string());
        map.insert("ui.preview".to_string(), "预览".to_string());
        map.insert("ui.add_param".to_string(), "添加参数".to_string());
        map.insert("ui.add_header".to_string(), "添加头部".to_string());
        map.insert("ui.add_form_data".to_string(), "添加表单数据".to_string());
        map.insert("ui.add_url_encoded".to_string(), "添加URL编码".to_string());
        map.insert("ui.none".to_string(), "none".to_string());
        map.insert("ui.form_data".to_string(), "form-data".to_string());
        map.insert("ui.url_encoded".to_string(), "x-www-form-urlencoded".to_string());
        map.insert("ui.binary".to_string(), "二进制".to_string());
        map.insert("ui.binary_placeholder".to_string(), "选择文件上传".to_string());
        map.insert("ui.no_auth".to_string(), "No Auth".to_string());
        map.insert("ui.bearer_token".to_string(), "Bearer Token".to_string());
        map.insert("ui.basic_auth".to_string(), "Basic Auth".to_string());
        map.insert("ui.api_key".to_string(), "API Key".to_string());
        map.insert("ui.token".to_string(), "Token".to_string());
        map.insert("ui.username".to_string(), "Username".to_string());
        map.insert("ui.password".to_string(), "Password".to_string());
        map.insert("ui.add_to".to_string(), "Add to".to_string());
        map.insert("ui.header".to_string(), "Header".to_string());
        map.insert("ui.query".to_string(), "Query".to_string());
        map.insert("ui.type".to_string(), "类型".to_string());
        map.insert("ui.click_send".to_string(), "点击发送按钮发送请求".to_string());
        map.insert("ui.no_env".to_string(), "无环境".to_string());
        map.insert("ui.online".to_string(), "在线".to_string());
        map.insert("ui.console".to_string(), "控制台".to_string());
        map.insert("ui.ready".to_string(), "就绪".to_string());

        // 响应
        map.insert("response.title".to_string(), "响应".to_string());
        map.insert("response.body".to_string(), "响应体".to_string());
        map.insert("response.headers".to_string(), "响应头".to_string());
        map.insert("response.cookies".to_string(), "Cookies".to_string());
        map.insert("response.test_results".to_string(), "测试结果".to_string());
        map.insert("response.status".to_string(), "状态码".to_string());
        map.insert("response.time".to_string(), "时间".to_string());
        map.insert("response.size".to_string(), "大小".to_string());
        map.insert("response.preview".to_string(), "预览".to_string());
        map.insert("response.raw".to_string(), "原始".to_string());
        map.insert("response.format".to_string(), "格式化".to_string());
        map.insert("response.formatted".to_string(), "格式化".to_string());
        map.insert("preview.not_available".to_string(), "无法预览此内容类型".to_string());
        map.insert("preview.open_in_browser".to_string(), "在浏览器中打开".to_string());
        map.insert("preview.view_source".to_string(), "查看源码".to_string());
        map.insert("preview.open_external".to_string(), "在外部程序中打开".to_string());

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
        map.insert("env.empty_list".to_string(), "还没有环境，先新建一个".to_string());
        map.insert("env.delete".to_string(), "删除环境".to_string());
        map.insert("env.vars_edit".to_string(), "编辑变量".to_string());
        map.insert("env.edit_vars_hint".to_string(), "点这里编辑该环境的变量".to_string());
        map.insert("env.edit".to_string(), "编辑环境".to_string());
        map.insert("env.create".to_string(), "新建环境".to_string());
        map.insert("env.save".to_string(), "保存".to_string());
        map.insert("env.save_env".to_string(), "保存环境".to_string());
        map.insert("env.cancel".to_string(), "取消".to_string());
        map.insert("env.name_label".to_string(), "环境名称".to_string());
        map.insert("env.current_vars".to_string(), "当前环境变量".to_string());
        map.insert("env.global_vars".to_string(), "全局变量".to_string());
        map.insert("env.empty_hint".to_string(), "暂无变量，点击右上角「添加变量」按钮添加".to_string());

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
        map.insert("code.copy_btn".to_string(), "复制到剪贴板".to_string());
        map.insert("code.close_btn".to_string(), "关闭".to_string());
        map.insert("clipboard.copied".to_string(), "已复制到剪贴板".to_string());
        map.insert("clipboard.copy_failed".to_string(), "复制失败".to_string());

        // 主题
        map.insert("theme.title".to_string(), "主题".to_string());
        map.insert("theme.light".to_string(), "晨曦".to_string());
        map.insert("theme.dark".to_string(), "暗夜".to_string());
        map.insert("theme.sepia".to_string(), "暖阳".to_string());
        map.insert("theme.ocean".to_string(), "海洋".to_string());
        map.insert("theme.sunset".to_string(), "日暮".to_string());
        map.insert("theme.forest".to_string(), "森林".to_string());
        map.insert("theme.monokai".to_string(), "摩卡".to_string());
        map.insert("theme.nord".to_string(), "北境".to_string());
        map.insert("theme.dracula".to_string(), "德古拉".to_string());
        map.insert("theme.tokyonight".to_string(), "东京夜".to_string());
        map.insert("theme.gruvbox".to_string(), "格鲁夫".to_string());
        map.insert("theme.latte".to_string(), "拿铁浅色".to_string());

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

        // 对话框
        map.insert("dialog.save_to_collections".to_string(), "保存到收藏夹".to_string());
        map.insert("dialog.title_label".to_string(), "标题".to_string());
        map.insert("dialog.cancel".to_string(), "取消".to_string());
        map.insert("dialog.new_folder".to_string(), "新建文件夹".to_string());
        map.insert("dialog.new_folder_hint".to_string(), "在收藏夹中创建一个新文件夹".to_string());
        map.insert("dialog.rename_folder".to_string(), "重命名文件夹".to_string());
        map.insert("dialog.rename_folder_hint".to_string(), "修改文件夹名称".to_string());
        map.insert("dialog.rename_request".to_string(), "重命名请求".to_string());
        map.insert("dialog.rename_request_hint".to_string(), "修改请求名称".to_string());
        map.insert("dialog.folder_name".to_string(), "文件夹名称".to_string());
        map.insert("dialog.request_name".to_string(), "请求名称".to_string());
        map.insert("dialog.save".to_string(), "保存".to_string());

        // 上下文菜单
        map.insert("context.rename".to_string(), "重命名".to_string());
        map.insert("context.move_to".to_string(), "移动到...".to_string());
        map.insert("context.add_subfolder".to_string(), "添加子文件夹".to_string());
        map.insert("context.delete".to_string(), "删除".to_string());
        map.insert("context.copy_curl".to_string(), "复制为 cURL".to_string());
        map.insert("context.generate_code".to_string(), "生成代码...".to_string());
        map.insert("context.copy_body".to_string(), "复制请求体".to_string());
        map.insert("context.copy_headers".to_string(), "复制请求头".to_string());
        map.insert("context.share_request".to_string(), "分享请求".to_string());

        // 侧边栏
        map.insert("sidebar.collections".to_string(), "收藏夹".to_string());
        map.insert("sidebar.collections_empty".to_string(), "暂无收藏请求，点击书签按钮保存请求".to_string());
        map.insert("sidebar.history".to_string(), "历史".to_string());
        map.insert("sidebar.env".to_string(), "环境变量".to_string());
        map.insert("sidebar.global_vars".to_string(), "全局变量".to_string());
        map.insert("sidebar.new_request".to_string(), "新建请求".to_string());

        // 设置
        map.insert("settings.title".to_string(), "设置".to_string());
        map.insert("settings.about".to_string(), "关于".to_string());
        map.insert("update.current".to_string(), "当前版本".to_string());
        map.insert("update.check".to_string(), "检查更新".to_string());
        map.insert("update.recheck".to_string(), "重新检查".to_string());
        map.insert("update.checking".to_string(), "正在检查更新…".to_string());
        map.insert("update.up_to_date".to_string(), "已是最新版本".to_string());
        map.insert("update.available".to_string(), "发现新版本".to_string());
        map.insert("update.download".to_string(), "下载新版本".to_string());
        map.insert("update.release_page".to_string(), "打开发布页".to_string());
        map.insert("update.failed".to_string(), "检查失败".to_string());
        map.insert("update.hint".to_string(), "从 GitHub Releases 获取最新版本".to_string());
        map.insert("update.published".to_string(), "发布于".to_string());
        map.insert("settings.timeout".to_string(), "超时时间".to_string());
        map.insert("settings.retries".to_string(), "重试次数".to_string());
        map.insert("settings.follow_redirects".to_string(), "跟随重定向".to_string());
        map.insert("settings.verify_ssl".to_string(), "验证 SSL 证书".to_string());
        map.insert("settings.unit_seconds".to_string(), "秒".to_string());
        map.insert("settings.unit_times".to_string(), "次".to_string());
        map.insert("auth.type".to_string(), "认证类型".to_string());
        map.insert("auth.none_hint".to_string(), "该请求不使用认证信息".to_string());
        map.insert("auth.token".to_string(), "Token".to_string());
        map.insert("auth.username".to_string(), "用户名".to_string());
        map.insert("auth.password".to_string(), "密码".to_string());
        map.insert("auth.key".to_string(), "键名".to_string());
        map.insert("auth.value".to_string(), "键值".to_string());
        map.insert("auth.add_to".to_string(), "添加到".to_string());
        map.insert("auth.bearer_hint".to_string(), "会在请求头加上 Authorization: Bearer <token>".to_string());
        map.insert("auth.basic_hint".to_string(), "使用 HTTP Basic 认证，用户名和密码会 Base64 编码后放进请求头".to_string());
        map.insert("auth.apikey_hint".to_string(), "把键值对添加到请求头或 URL 查询参数".to_string());
        map.insert("auth.effective".to_string(), "生效内容".to_string());
        map.insert("env.vars".to_string(), "变量".to_string());
        map.insert("env.vars_count".to_string(), "{} 个变量".to_string());
        map.insert("env.no_vars".to_string(), "该环境暂无变量".to_string());
        map.insert("env.name_required".to_string(), "环境名称不能为空".to_string());
        map.insert("settings.general".to_string(), "常规".to_string());
        map.insert("settings.auto_save".to_string(), "自动保存".to_string());
        map.insert("settings.proxy".to_string(), "代理".to_string());
        map.insert("settings.proxy_enable".to_string(), "启用代理".to_string());
        map.insert("settings.proxy_url".to_string(), "代理地址".to_string());
        map.insert("settings.proxy_tips".to_string(), "格式: http://host:port 或 socks5://host:port".to_string());
        map.insert("settings.not_set".to_string(), "未设置".to_string());
        map.insert("settings.shortcuts".to_string(), "快捷键".to_string());
        map.insert("settings.shortcuts.send".to_string(), "发送请求".to_string());
        map.insert("settings.shortcuts.new_tab".to_string(), "新建标签页".to_string());
        map.insert("settings.shortcuts.close_tab".to_string(), "关闭标签页".to_string());
        map.insert("settings.shortcuts.history".to_string(), "打开历史记录".to_string());
        map.insert("settings.shortcuts.env".to_string(), "打开环境变量".to_string());
        map.insert("settings.shortcuts.theme".to_string(), "切换主题".to_string());
        map.insert("settings.shortcuts.lang".to_string(), "切换语言".to_string());

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
        map.insert("request.pre_request".to_string(), "Pre-request".to_string());
        map.insert("request.tests".to_string(), "Tests".to_string());
        map.insert("request.settings".to_string(), "Settings".to_string());
        map.insert("ui.key".to_string(), "Key".to_string());
        map.insert("ui.value".to_string(), "Value".to_string());
        map.insert("ui.send".to_string(), "Send".to_string());
        map.insert("ui.sending".to_string(), "Sending...".to_string());
        map.insert("ui.no_history".to_string(), "No history yet".to_string());
        map.insert("ui.json".to_string(), "JSON".to_string());
        map.insert("ui.xml".to_string(), "XML".to_string());
        map.insert("ui.text".to_string(), "Text".to_string());
        map.insert("ui.html".to_string(), "HTML".to_string());
        map.insert("ui.pretty".to_string(), "Pretty".to_string());
        map.insert("ui.raw".to_string(), "Raw".to_string());
        map.insert("ui.preview".to_string(), "Preview".to_string());
        map.insert("ui.add_param".to_string(), "Add parameter".to_string());
        map.insert("ui.add_header".to_string(), "Add Header".to_string());
        map.insert("ui.add_form_data".to_string(), "Add form data".to_string());
        map.insert("ui.add_url_encoded".to_string(), "Add URL-encoded".to_string());
        map.insert("ui.none".to_string(), "none".to_string());
        map.insert("ui.form_data".to_string(), "form-data".to_string());
        map.insert("ui.url_encoded".to_string(), "x-www-form-urlencoded".to_string());
        map.insert("ui.binary".to_string(), "binary".to_string());
        map.insert("ui.binary_placeholder".to_string(), "Select a file to upload".to_string());
        map.insert("ui.no_auth".to_string(), "No Auth".to_string());
        map.insert("ui.bearer_token".to_string(), "Bearer Token".to_string());
        map.insert("ui.basic_auth".to_string(), "Basic Auth".to_string());
        map.insert("ui.api_key".to_string(), "API Key".to_string());
        map.insert("ui.token".to_string(), "Token".to_string());
        map.insert("ui.username".to_string(), "Username".to_string());
        map.insert("ui.password".to_string(), "Password".to_string());
        map.insert("ui.add_to".to_string(), "Add to".to_string());
        map.insert("ui.header".to_string(), "Header".to_string());
        map.insert("ui.query".to_string(), "Query".to_string());
        map.insert("ui.type".to_string(), "Type".to_string());
        map.insert("ui.click_send".to_string(), "Click Send to request".to_string());
        map.insert("ui.no_env".to_string(), "No Environment".to_string());
        map.insert("ui.online".to_string(), "Online".to_string());
        map.insert("ui.console".to_string(), "Console".to_string());
        map.insert("ui.ready".to_string(), "Ready".to_string());

        // Response
        map.insert("response.title".to_string(), "Response".to_string());
        map.insert("response.body".to_string(), "Body".to_string());
        map.insert("response.headers".to_string(), "Headers".to_string());
        map.insert("response.cookies".to_string(), "Cookies".to_string());
        map.insert("response.test_results".to_string(), "Test Results".to_string());
        map.insert("response.status".to_string(), "Status".to_string());
        map.insert("response.time".to_string(), "Time".to_string());
        map.insert("response.size".to_string(), "Size".to_string());
        map.insert("response.preview".to_string(), "Preview".to_string());
        map.insert("response.raw".to_string(), "Raw".to_string());
        map.insert("response.format".to_string(), "Format".to_string());
        map.insert("response.formatted".to_string(), "Formatted".to_string());
        map.insert("preview.not_available".to_string(), "Preview not available for this content type".to_string());
        map.insert("preview.open_in_browser".to_string(), "Open in Browser".to_string());
        map.insert("preview.view_source".to_string(), "View Source".to_string());
        map.insert("preview.open_external".to_string(), "Open in External Program".to_string());

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
        map.insert("env.name".to_string(), "Variable".to_string());
        map.insert("env.value".to_string(), "Current Value".to_string());
        map.insert("env.no_env".to_string(), "No environment selected".to_string());
        map.insert("env.empty_list".to_string(), "No environments yet — create one".to_string());
        map.insert("env.delete".to_string(), "Delete Environment".to_string());
        map.insert("env.vars_edit".to_string(), "Edit Variables".to_string());
        map.insert("env.edit_vars_hint".to_string(), "Click to edit this environment's variables".to_string());
        map.insert("env.edit".to_string(), "Edit Environment".to_string());
        map.insert("env.create".to_string(), "New Environment".to_string());
        map.insert("env.save".to_string(), "Save".to_string());
        map.insert("env.save_env".to_string(), "Save Environment".to_string());
        map.insert("env.cancel".to_string(), "Cancel".to_string());
        map.insert("env.name_label".to_string(), "Environment Name".to_string());
        map.insert("env.current_vars".to_string(), "Current Variables".to_string());
        map.insert("env.global_vars".to_string(), "Global Variables".to_string());
        map.insert("env.empty_hint".to_string(), "No variables. Click 'Add Variable' to add".to_string());

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
        map.insert("code.copy_btn".to_string(), "Copy to Clipboard".to_string());
        map.insert("code.close_btn".to_string(), "Close".to_string());
        map.insert("clipboard.copied".to_string(), "Copied to clipboard".to_string());
        map.insert("clipboard.copy_failed".to_string(), "Copy failed".to_string());

        // Theme
        map.insert("theme.title".to_string(), "Theme".to_string());
        map.insert("theme.light".to_string(), "Light".to_string());
        map.insert("theme.dark".to_string(), "Dark".to_string());
        map.insert("theme.sepia".to_string(), "Sepia".to_string());
        map.insert("theme.ocean".to_string(), "Ocean".to_string());
        map.insert("theme.sunset".to_string(), "Sunset".to_string());
        map.insert("theme.forest".to_string(), "Forest".to_string());
        map.insert("theme.monokai".to_string(), "Monokai".to_string());
        map.insert("theme.nord".to_string(), "Nord".to_string());
        map.insert("theme.dracula".to_string(), "Dracula".to_string());
        map.insert("theme.tokyonight".to_string(), "Tokyo".to_string());
        map.insert("theme.gruvbox".to_string(), "Gruvbox".to_string());
        map.insert("theme.latte".to_string(), "Latte".to_string());

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

        // Dialog
        map.insert("dialog.save_to_collections".to_string(), "Save to Collections".to_string());
        map.insert("dialog.title_label".to_string(), "Title".to_string());
        map.insert("dialog.cancel".to_string(), "Cancel".to_string());
        map.insert("dialog.new_folder".to_string(), "New Folder".to_string());
        map.insert("dialog.new_folder_hint".to_string(), "Create a new folder in your collections".to_string());
        map.insert("dialog.rename_folder".to_string(), "Rename Folder".to_string());
        map.insert("dialog.rename_folder_hint".to_string(), "Change the folder name".to_string());
        map.insert("dialog.rename_request".to_string(), "Rename Request".to_string());
        map.insert("dialog.rename_request_hint".to_string(), "Change the request name".to_string());
        map.insert("dialog.folder_name".to_string(), "Folder Name".to_string());
        map.insert("dialog.request_name".to_string(), "Request Name".to_string());
        map.insert("dialog.save".to_string(), "Save".to_string());

        // Context Menu
        map.insert("context.rename".to_string(), "Rename".to_string());
        map.insert("context.move_to".to_string(), "Move to...".to_string());
        map.insert("context.add_subfolder".to_string(), "Add Subfolder".to_string());
        map.insert("context.delete".to_string(), "Delete".to_string());
        map.insert("context.copy_curl".to_string(), "Copy as cURL".to_string());
        map.insert("context.generate_code".to_string(), "Generate Code...".to_string());
        map.insert("context.copy_body".to_string(), "Copy Request Body".to_string());
        map.insert("context.copy_headers".to_string(), "Copy Request Headers".to_string());
        map.insert("context.share_request".to_string(), "Share Request".to_string());

        // Sidebar
        map.insert("sidebar.collections".to_string(), "Collections".to_string());
        map.insert("sidebar.collections_empty".to_string(), "No saved requests. Click the bookmark button to save.".to_string());
        map.insert("sidebar.history".to_string(), "History".to_string());
        map.insert("sidebar.env".to_string(), "Environment".to_string());
        map.insert("sidebar.global_vars".to_string(), "Global Variables".to_string());
        map.insert("sidebar.new_request".to_string(), "New Request".to_string());

        // Settings
        map.insert("settings.title".to_string(), "Settings".to_string());
        map.insert("settings.about".to_string(), "About".to_string());
        map.insert("update.current".to_string(), "Current version".to_string());
        map.insert("update.check".to_string(), "Check for updates".to_string());
        map.insert("update.recheck".to_string(), "Check again".to_string());
        map.insert("update.checking".to_string(), "Checking for updates…".to_string());
        map.insert("update.up_to_date".to_string(), "Up to date".to_string());
        map.insert("update.available".to_string(), "New version available".to_string());
        map.insert("update.download".to_string(), "Download".to_string());
        map.insert("update.release_page".to_string(), "Open release page".to_string());
        map.insert("update.failed".to_string(), "Check failed".to_string());
        map.insert("update.hint".to_string(), "Fetch the latest version from GitHub Releases".to_string());
        map.insert("update.published".to_string(), "Published".to_string());
        map.insert("settings.timeout".to_string(), "Timeout".to_string());
        map.insert("settings.retries".to_string(), "Retries".to_string());
        map.insert("settings.follow_redirects".to_string(), "Follow Redirects".to_string());
        map.insert("settings.verify_ssl".to_string(), "Verify SSL Certificate".to_string());
        map.insert("settings.unit_seconds".to_string(), "s".to_string());
        map.insert("settings.unit_times".to_string(), "times".to_string());
        map.insert("auth.type".to_string(), "Auth Type".to_string());
        map.insert("auth.none_hint".to_string(), "This request does not use any authorization.".to_string());
        map.insert("auth.token".to_string(), "Token".to_string());
        map.insert("auth.username".to_string(), "Username".to_string());
        map.insert("auth.password".to_string(), "Password".to_string());
        map.insert("auth.key".to_string(), "Key".to_string());
        map.insert("auth.value".to_string(), "Value".to_string());
        map.insert("auth.add_to".to_string(), "Add To".to_string());
        map.insert("auth.bearer_hint".to_string(), "Adds an Authorization: Bearer <token> header".to_string());
        map.insert("auth.basic_hint".to_string(), "Uses HTTP Basic auth; the credentials are Base64 encoded".to_string());
        map.insert("auth.apikey_hint".to_string(), "Adds the key/value pair to the request header or URL query".to_string());
        map.insert("auth.effective".to_string(), "Effective".to_string());
        map.insert("env.vars".to_string(), "Variables".to_string());
        map.insert("env.vars_count".to_string(), "{} variables".to_string());
        map.insert("env.no_vars".to_string(), "No variables in this environment".to_string());
        map.insert("env.name_required".to_string(), "Environment name is required".to_string());
        map.insert("settings.general".to_string(), "General".to_string());
        map.insert("settings.auto_save".to_string(), "Auto Save".to_string());
        map.insert("settings.proxy".to_string(), "Proxy".to_string());
        map.insert("settings.proxy_enable".to_string(), "Enable Proxy".to_string());
        map.insert("settings.proxy_url".to_string(), "Proxy URL".to_string());
        map.insert("settings.proxy_tips".to_string(), "Format: http://host:port or socks5://host:port".to_string());
        map.insert("settings.not_set".to_string(), "Not set".to_string());
        map.insert("settings.shortcuts".to_string(), "Shortcuts".to_string());
        map.insert("settings.shortcuts.send".to_string(), "Send Request".to_string());
        map.insert("settings.shortcuts.new_tab".to_string(), "New Tab".to_string());
        map.insert("settings.shortcuts.close_tab".to_string(), "Close Tab".to_string());
        map.insert("settings.shortcuts.history".to_string(), "Open History".to_string());
        map.insert("settings.shortcuts.env".to_string(), "Open Environments".to_string());
        map.insert("settings.shortcuts.theme".to_string(), "Cycle Theme".to_string());
        map.insert("settings.shortcuts.lang".to_string(), "Toggle Language".to_string());

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

    /// 表单/表格列标题必须中英都有（form-data 的 键/类型/值、urlencoded 与响应头的 键/值）
    #[test]
    fn test_table_header_keys_exist_in_both_languages() {
        for lang in ["zh-CN", "en-US"] {
            let i18n = I18nManager::new(lang);
            for key in ["ui.key", "ui.value", "ui.type"] {
                assert_ne!(i18n.get(key), key, "{lang} 缺少翻译键：{key}");
            }
        }
        // 中文下 ui.type 不能还是英文 "Type"
        let zh = I18nManager::new("zh-CN");
        assert_eq!(zh.get("ui.type"), "类型");
        assert_eq!(zh.get("ui.key"), "键");
        assert_eq!(zh.get("ui.value"), "值");
    }

    /// 「关于 / 更新检查」用到的键中英必须都齐 —— 少一个界面上就直接显示键名了。
    /// （get() 取不到时会原样返回 key，正好拿来判断）
    #[test]
    fn test_update_keys_exist_in_both_languages() {
        let keys = [
            "settings.about",
            "update.current",
            "update.check",
            "update.recheck",
            "update.checking",
            "update.up_to_date",
            "update.available",
            "update.download",
            "update.release_page",
            "update.failed",
            "update.hint",
            "update.published",
        ];
        for lang in ["zh-CN", "en-US"] {
            let i18n = I18nManager::new(lang);
            for key in keys {
                assert_ne!(i18n.get(key), key, "{lang} 缺少翻译键：{key}");
            }
        }
    }
}