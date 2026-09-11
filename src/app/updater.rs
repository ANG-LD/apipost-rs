//! 自动更新检查：查 GitHub Releases 里的最新 release，和本地版本做对比。
//!
//! 设计要点：
//! * 只看 `releases/latest` —— GitHub 这个接口本身就会跳过 draft 和 prerelease，
//!   所以 CI 里打了 `v0.3.0-beta.1` 这种 tag 并标了 prerelease，就不会被当成"最新版"。
//! * 版本号来源是 `CARGO_PKG_VERSION`（即 Cargo.toml 的 version），仓库地址取自
//!   `CARGO_PKG_REPOSITORY`，两边只有一处需要维护。
//! * 纯逻辑（版本比较、平台资源挑选）都拆成了可单测的纯函数，网络部分只负责取 JSON。

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::time::Duration;

/// 本地版本（Cargo.toml 的 version）
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cargo.toml 里的 repository 字段
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// 解析 owner/repo。解析不出来时退回硬编码，保证检查功能不会因为元数据写错就失效。
pub fn repo_slug() -> String {
    let url = REPOSITORY
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    if let Some(rest) = url.split("github.com/").nth(1) {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[0], parts[1]);
        }
    }
    "ANG-LD/apipost-rs".to_string()
}

/// GitHub release 里的一个附件
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    #[serde(default)]
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

impl ReleaseAsset {
    /// 人类可读的大小
    pub fn size_text(&self) -> String {
        let kb = self.size as f64 / 1024.0;
        if kb < 1024.0 {
            format!("{kb:.0} KB")
        } else {
            format!("{:.1} MB", kb / 1024.0)
        }
    }
}

/// `GET /repos/{owner}/{repo}/releases/latest` 的响应（只取用得到的字段）
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseInfo {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseInfo {
    /// tag 名规范化后的版本号，如 "v0.2.0" -> "0.2.0"
    pub fn version(&self) -> String {
        normalize_version(&self.tag_name)
    }

    /// 当前平台该下哪个附件（没有合适的就返回 None，界面退化成"打开发布页"）
    pub fn asset_for_platform(&self) -> Option<&ReleaseAsset> {
        pick_asset(&self.assets)
    }

    /// 发布日期只取日期部分
    pub fn published_date(&self) -> String {
        self.published_at
            .split('T')
            .next()
            .unwrap_or_default()
            .to_string()
    }
}

/// 更新检查状态（直接挂在 MainView 上给界面用）
#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    /// 还没检查过
    #[default]
    Idle,
    /// 正在检查
    Checking,
    /// 已是最新
    UpToDate { latest: String },
    /// 有新版本
    Available(Box<ReleaseInfo>),
    /// 检查失败（网络不通、限流等），文案直接展示
    Failed(String),
}

impl UpdateStatus {
    pub fn is_checking(&self) -> bool {
        matches!(self, UpdateStatus::Checking)
    }
}

/// 去掉 tag 前缀，只留版本号
pub fn normalize_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches(['v', 'V'])
        .trim()
        .to_string()
}

/// 把 "1.2.3" / "1.2.3-beta.1" 拆成 (major, minor, patch, 预发布串)。
/// 解析不了返回 None —— 调用方按"无法比较"处理，不会误报有新版本。
pub fn parse_version(v: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let v = normalize_version(v);
    // 版本主体和预发布部分用第一个 '-' 或 '+' 分开
    let (core, pre) = match v.find(['-', '+']) {
        Some(i) => (&v[..i], Some(v[i + 1..].to_string())),
        None => (v.as_str(), None),
    };
    let mut nums = core.split('.');
    let major = nums.next()?.trim().parse::<u64>().ok()?;
    let minor = nums.next().unwrap_or("0").trim().parse::<u64>().unwrap_or(0);
    let patch = nums.next().unwrap_or("0").trim().parse::<u64>().unwrap_or(0);
    // 主体里出现非数字（例如 "nightly"）就当解析失败
    if core
        .split('.')
        .any(|part| !part.trim().chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some((major, minor, patch, pre.filter(|p| !p.is_empty())))
}

/// 远端版本是否比本地新。
///
/// 规则（够用且不会误报）：
/// * 主版本号三元组大 → 新
/// * 三元组相同：远端是正式版、本地是预发布 → 新（1.0.0 > 1.0.0-beta）
/// * 其它（含两边解析不出来）→ 不算新
pub fn is_newer(remote: &str, local: &str) -> bool {
    match (parse_version(remote), parse_version(local)) {
        (Some((rm, rn, rp, rpre)), Some((lm, ln, lp, lpre))) => {
            match (rm, rn, rp).cmp(&(lm, ln, lp)) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => rpre.is_none() && lpre.is_some(),
            }
        }
        _ => false,
    }
}

/// 按当前平台挑最合适的安装包：优先安装器，其次原始二进制
pub fn pick_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    // 依次尝试的关键字（小写匹配文件名结尾或包含关系）
    let preferences: &[&str] = if cfg!(target_os = "windows") {
        &["setup.exe", ".exe", "windows"]
    } else if cfg!(target_os = "macos") {
        &[".dmg", "apple-darwin", "macos"]
    } else {
        &[".deb", ".appimage", "linux"]
    };
    for key in preferences {
        if let Some(a) = assets
            .iter()
            .find(|a| a.name.to_lowercase().contains(&key.to_lowercase()))
        {
            return Some(a);
        }
    }
    None
}

/// 查最新 release 并给出结论（网络请求在这里，别的都是纯函数）
pub async fn check(proxy: Option<String>) -> UpdateStatus {
    log::info!(
        "开始检查更新: 本地 v{CURRENT_VERSION}, 仓库 {}",
        repo_slug()
    );
    match fetch_latest_release(proxy).await {
        Ok(info) => {
            let remote = info.version();
            if is_newer(&remote, CURRENT_VERSION) {
                log::info!("检查更新: 发现新版本 v{remote}（本地 v{CURRENT_VERSION}）");
                UpdateStatus::Available(Box::new(info))
            } else {
                log::info!("检查更新: 已是最新（远端 v{remote}，本地 v{CURRENT_VERSION}）");
                UpdateStatus::UpToDate { latest: remote }
            }
        }
        Err(e) => {
            log::warn!("检查更新失败: {e:#}");
            UpdateStatus::Failed(format!("{e:#}"))
        }
    }
}

/// 拉取本仓库的最新 release
pub async fn fetch_latest_release(proxy: Option<String>) -> Result<ReleaseInfo> {
    fetch_latest_release_for(&repo_slug(), proxy).await
}

/// 拉取指定仓库的最新 release。GitHub API 强制要求 User-Agent，不带会直接 403。
pub async fn fetch_latest_release_for(repo: &str, proxy: Option<String>) -> Result<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let mut builder = reqwest::Client::builder()
        .user_agent(format!("apipost-rs/{CURRENT_VERSION}"))
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(8));
    if let Some(p) = proxy.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        // 代理填错不应该让整个检查崩掉，退化成直连更有用
        match reqwest::Proxy::all(p) {
            Ok(proxy) => builder = builder.proxy(proxy),
            Err(e) => log::warn!("更新检查：代理地址无效（{p}）：{e}，改为直连"),
        }
    }
    let client = builder.build().context("创建 HTTP 客户端失败")?;

    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("请求 GitHub 失败（检查网络/代理）")?;

    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("没有找到 release（{url}）");
    }
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        bail!("GitHub 限流（{status}），稍后再试");
    }
    if !status.is_success() {
        bail!("GitHub 返回 {status}");
    }

    resp.json::<ReleaseInfo>()
        .await
        .context("解析 GitHub 返回内容失败")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_v_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("V1.2.3"), "1.2.3");
        assert_eq!(normalize_version(" 1.2.3 "), "1.2.3");
    }

    #[test]
    fn parse_version_handles_common_forms() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3, None)));
        assert_eq!(parse_version("v1.2"), Some((1, 2, 0, None)));
        assert_eq!(
            parse_version("1.2.3-beta.1"),
            Some((1, 2, 3, Some("beta.1".to_string())))
        );
        assert_eq!(parse_version("nightly"), None);
    }

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        // 本地是预发布，远端是同一版的正式版 -> 算有新版本
        assert!(is_newer("1.0.0", "1.0.0-beta.1"));
        // 反过来不算
        assert!(!is_newer("1.0.0-beta.1", "1.0.0"));
        // 解析不了时不误报
        assert!(!is_newer("", "0.1.0"));
        assert!(!is_newer("abc", "0.1.0"));
    }

    #[test]
    fn repo_slug_comes_from_metadata() {
        // Cargo.toml 里写的是完整 URL，这里应该只剩 owner/repo
        let slug = repo_slug();
        assert!(slug.contains('/'), "slug 应该是 owner/repo 形式：{slug}");
        assert!(!slug.contains("github.com"), "slug 不该带域名：{slug}");
    }

    #[test]
    fn pick_asset_prefers_installer() {
        let assets = vec![
            ReleaseAsset {
                name: "apipost-rs-x86_64-unknown-linux-gnu".into(),
                browser_download_url: "https://example.com/raw".into(),
                size: 1024,
            },
            ReleaseAsset {
                name: "apipost-rs_0.1.0_amd64.deb".into(),
                browser_download_url: "https://example.com/deb".into(),
                size: 2048,
            },
        ];
        let picked = pick_asset(&assets).unwrap();
        if cfg!(target_os = "linux") {
            assert!(picked.name.ends_with(".deb"));
        } else {
            assert!(!picked.name.is_empty());
        }
        assert_eq!(assets[0].size_text(), "1 KB");
    }

    /// 联网冒烟测试：手动执行 `cargo test -- --ignored live_check` 验证真实 API 路径。
    /// 平时不跑（CI 无网/限流会误报）。
    #[test]
    #[ignore]
    fn live_check_hits_github() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let info = rt
            .block_on(fetch_latest_release_for("zed-industries/zed", None))
            .expect("联网拉取失败");
        assert!(!info.tag_name.is_empty(), "tag 不该为空");
        assert!(parse_version(&info.version()).is_some(), "版本号要能解析");
    }

    #[test]
    fn deserializes_github_payload() {
        // 真实 GitHub 响应裁剪后的样子（字段名要和 API 对齐）
        let json = r#"{
            "tag_name": "v0.2.0",
            "name": "ApiPost-Rs v0.2.0",
            "html_url": "https://github.com/ANG-LD/apipost-rs/releases/tag/v0.2.0",
            "body": "- 新增自动更新检查",
            "published_at": "2025-09-10T08:00:00Z",
            "prerelease": false,
            "assets": [
                {"name": "apipost-rs-x86_64-pc-windows-msvc.exe",
                 "browser_download_url": "https://example.com/win.exe", "size": 20480}
            ]
        }"#;
        let info: ReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version(), "0.2.0");
        assert_eq!(info.published_date(), "2025-09-10");
        assert_eq!(info.assets.len(), 1);
        assert_eq!(info.asset_for_platform().map(|a| a.size_text()), {
            if cfg!(target_os = "windows") {
                Some("20 KB".to_string())
            } else {
                // 非 Windows 平台挑不到 exe（关键字不匹配），界面会退化成打开发布页
                None
            }
        });
    }
}
