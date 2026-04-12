//! 配置管理模块
//!
//! 负责应用配置的加载、保存和访问
//! 配置文件位于: ~/.config/apipost-rs/config.toml

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用配置结构
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// 通用配置
    pub general: GeneralConfig,
    /// 代理配置
    pub proxy: ProxyConfig,
    /// 数据库配置
    pub database: DatabaseConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            proxy: ProxyConfig::default(),
            database: DatabaseConfig::default(),
        }
    }
}

/// 通用配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    /// 语言设置 (zh-CN, en-US)
    pub language: String,
    /// 主题设置 (light, dark)
    pub theme: String,
    /// 自动保存
    pub auto_save: bool,
    /// 超时时间（秒）
    pub timeout: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "dark".to_string(),
            auto_save: true,
            timeout: 30,
        }
    }
}

/// 代理配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// 是否启用代理
    pub enabled: bool,
    /// 代理地址
    pub url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
        }
    }
}

/// 数据库配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// 数据库路径
    pub path: String,
    /// 备份间隔（秒）
    pub backup_interval: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "~/.local/share/apipost-rs/database.db".to_string(),
            backup_interval: 3600,
        }
    }
}

impl AppConfig {
    /// 获取配置文件路径
    fn get_config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "apipost-rs", "apipost-rs")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// 从配置文件加载配置
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::get_config_path()
            .ok_or_else(|| anyhow::anyhow!("无法确定配置文件目录"))?;

        if !path.exists() {
            let config = AppConfig::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_config_path()
            .ok_or_else(|| anyhow::anyhow!("无法确定配置文件目录"))?;

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.general.language, "zh-CN");
        assert_eq!(config.general.theme, "dark");
        assert_eq!(config.general.timeout, 30);
    }

    #[test]
    fn test_general_config_default() {
        let general = GeneralConfig::default();
        assert_eq!(general.language, "zh-CN");
        assert_eq!(general.theme, "dark");
        assert!(general.auto_save);
    }
}
