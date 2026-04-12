//! 环境变量管理模块
//!
//! 负责管理全局和局部环境变量
//! 支持变量替换语法: {{variable_name}}

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// 环境变量管理器
pub struct EnvironmentManager {
    /// 全局变量（所有环境共享）
    global_variables: RwLock<HashMap<String, String>>,
    /// 当前环境的变量
    current_variables: RwLock<HashMap<String, String>>,
    /// 变量替换正则
    variable_pattern: Regex,
}

impl Clone for EnvironmentManager {
    fn clone(&self) -> Self {
        Self {
            global_variables: RwLock::new(self.global_variables.read().unwrap().clone()),
            current_variables: RwLock::new(self.current_variables.read().unwrap().clone()),
            variable_pattern: Regex::new(r"\{\{([^}]+)\}\}").unwrap(),
        }
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentManager {
    /// 创建新的环境变量管理器
    pub fn new() -> Self {
        Self {
            global_variables: RwLock::new(HashMap::new()),
            current_variables: RwLock::new(HashMap::new()),
            // 匹配 {{variable_name}} 格式的变量
            variable_pattern: Regex::new(r"\{\{([^}]+)\}\}").unwrap(),
        }
    }

    /// 设置全局变量
    pub fn set_global(&self, key: String, value: String) {
        let mut globals = self.global_variables.write().unwrap();
        globals.insert(key, value);
    }

    /// 获取全局变量
    pub fn get_global(&self, key: &str) -> Option<String> {
        let globals = self.global_variables.read().unwrap();
        globals.get(key).cloned()
    }

    /// 获取所有全局变量
    pub fn get_all_globals(&self) -> HashMap<String, String> {
        let globals = self.global_variables.read().unwrap();
        globals.clone()
    }

    /// 设置当前环境变量
    pub fn set_current(&self, key: String, value: String) {
        let mut current = self.current_variables.write().unwrap();
        current.insert(key, value);
    }

    /// 获取当前环境变量
    pub fn get_current(&self, key: &str) -> Option<String> {
        let current = self.current_variables.read().unwrap();
        current.get(key).cloned()
    }

    /// 获取所有当前环境变量
    pub fn get_all_current(&self) -> HashMap<String, String> {
        let current = self.current_variables.read().unwrap();
        current.clone()
    }

    /// 清空当前环境变量
    pub fn clear_current(&self) {
        let mut current = self.current_variables.write().unwrap();
        current.clear();
    }

    /// 从JSON字符串加载变量
    pub fn load_from_json(&self, json: &str) -> Result<()> {
        let vars: HashMap<String, String> = serde_json::from_str(json)?;
        let mut current = self.current_variables.write().unwrap();
        *current = vars;
        Ok(())
    }

    /// 导出变量为JSON字符串
    pub fn export_to_json(&self) -> Result<String> {
        let current = self.current_variables.read().unwrap();
        Ok(serde_json::to_string(&*current)?)
    }

    /// 替换字符串中的变量
    ///
    /// 语法: {{variable_name}}
    /// 优先级: 当前环境变量 > 全局变量
    ///
    /// # 性能说明
    /// 使用单次遍历构建结果，避免每次替换都分配新字符串
    pub fn replace_variables(&self, input: &str) -> String {
        let globals = self.global_variables.read().unwrap();
        let current = self.current_variables.read().unwrap();

        let mut result = String::with_capacity(input.len());
        let mut last_end = 0;

        // 一次遍历完成替换，避免 O(n*m) 复杂度
        for cap in self.variable_pattern.captures_iter(input) {
            let full_match = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();

            // 添加匹配位置之前的文本
            result.push_str(&input[last_end..full_match.start()]);

            // 查找替换值：优先当前环境，其次全局环境，最后保持原样
            let replacement = current
                .get(var_name)
                .or_else(|| globals.get(var_name))
                .map(|s| s.as_str())
                .unwrap_or(full_match.as_str());
            result.push_str(replacement);

            last_end = full_match.end();
        }

        // 添加剩余未匹配的文本
        result.push_str(&input[last_end..]);
        result
    }

    /// 检查字符串是否包含变量
    pub fn contains_variables(&self, input: &str) -> bool {
        self.variable_pattern.is_match(input)
    }

    /// 提取字符串中的所有变量名
    pub fn extract_variables(&self, input: &str) -> Vec<String> {
        self.variable_pattern
            .captures_iter(input)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }

    /// 批量设置变量
    pub fn set_variables(&self, vars: HashMap<String, String>) {
        let mut current = self.current_variables.write().unwrap();
        *current = vars;
    }

    /// 设置全局变量（批量）
    pub fn set_globals(&self, vars: HashMap<String, String>) {
        let mut global = self.global_variables.write().unwrap();
        *global = vars;
    }
}

/// 环境变量数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    /// 变量名
    pub key: String,
    /// 变量值
    pub value: String,
    /// 是否启用
    pub enabled: bool,
}

/// 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    /// 环境名称
    pub name: String,
    /// 环境变量列表
    pub variables: Vec<EnvVariable>,
}

impl EnvConfig {
    /// 从JSON加载
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 导出为JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 转换为HashMap
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.variables
            .iter()
            .filter(|v| v.enabled)
            .map(|v| (v.key.clone(), v.value.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> EnvironmentManager {
        let manager = EnvironmentManager::new();
        manager.set_global("global_var".to_string(), "global_value".to_string());
        manager.set_current("base_url".to_string(), "https://api.example.com".to_string());
        manager.set_current("api_key".to_string(), "test_key_123".to_string());
        manager
    }

    #[test]
    fn test_set_and_get_variable() {
        let manager = EnvironmentManager::new();
        manager.set_current("name".to_string(), "test".to_string());
        assert_eq!(manager.get_current("name"), Some("test".to_string()));
    }

    #[test]
    fn test_replace_simple_variable() {
        let manager = create_test_manager();
        let input = "{{base_url}}/users";
        let result = manager.replace_variables(input);
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_replace_multiple_variables() {
        let manager = create_test_manager();
        let input = "{{base_url}}/users?api_key={{api_key}}";
        let result = manager.replace_variables(input);
        assert_eq!(result, "https://api.example.com/users?api_key=test_key_123");
    }

    #[test]
    fn test_replace_global_variable() {
        let manager = create_test_manager();
        let input = "Global: {{global_var}}";
        let result = manager.replace_variables(input);
        assert_eq!(result, "Global: global_value");
    }

    #[test]
    fn test_replace_priority() {
        let manager = create_test_manager();
        // 设置同名的全局和当前变量，当前变量应该优先
        manager.set_global("test".to_string(), "global".to_string());
        manager.set_current("test".to_string(), "current".to_string());

        let input = "{{test}}";
        let result = manager.replace_variables(input);
        assert_eq!(result, "current");
    }

    #[test]
    fn test_replace_no_variable() {
        let manager = create_test_manager();
        let input = "No variables here";
        let result = manager.replace_variables(input);
        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_replace_unknown_variable() {
        let manager = create_test_manager();
        // 未知变量应该保持原样
        let input = "{{unknown_var}}";
        let result = manager.replace_variables(input);
        assert_eq!(result, "{{unknown_var}}");
    }

    #[test]
    fn test_contains_variables() {
        let manager = create_test_manager();
        assert!(manager.contains_variables("{{base_url}}"));
        assert!(!manager.contains_variables("no variables"));
    }

    #[test]
    fn test_extract_variables() {
        let manager = create_test_manager();
        let input = "{{base_url}}/{{path}}/{{id}}";
        let vars = manager.extract_variables(input);
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"base_url".to_string()));
        assert!(vars.contains(&"path".to_string()));
        assert!(vars.contains(&"id".to_string()));
    }

    #[test]
    fn test_load_from_json() {
        let manager = EnvironmentManager::new();
        let json = r#"{"key1":"value1","key2":"value2"}"#;
        manager.load_from_json(json).unwrap();

        assert_eq!(manager.get_current("key1"), Some("value1".to_string()));
        assert_eq!(manager.get_current("key2"), Some("value2".to_string()));
    }

    #[test]
    fn test_export_to_json() {
        let manager = EnvironmentManager::new();
        manager.set_current("key1".to_string(), "value1".to_string());
        manager.set_current("key2".to_string(), "value2".to_string());

        let json = manager.export_to_json().unwrap();
        assert!(json.contains("key1"));
        assert!(json.contains("value1"));
    }

    #[test]
    fn test_env_config() {
        let config = EnvConfig {
            name: "Test Env".to_string(),
            variables: vec![
                EnvVariable {
                    key: "base_url".to_string(),
                    value: "https://test.com".to_string(),
                    enabled: true,
                },
                EnvVariable {
                    key: "disabled_var".to_string(),
                    value: "should not appear".to_string(),
                    enabled: false,
                },
            ],
        };

        let map = config.to_hashmap();
        assert_eq!(map.get("base_url"), Some(&"https://test.com".to_string()));
        assert!(!map.contains_key("disabled_var"));
    }
}
