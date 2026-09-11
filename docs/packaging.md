# 打包与安装（Windows / macOS / Linux）

打包统一走 `cargo-packager`，配置在仓库根目录的 `packager.toml`。
本文记录各平台产物、图标来源，以及"装完桌面上没图标"这类坑的成因。

## 一、图标：三个格式缺一不可

| 平台 | 需要的文件 | 用在哪 |
| --- | --- | --- |
| Windows | `assets/icon.ico` | ① `build.rs` 写进 exe 的图标资源；② NSIS 安装器自身图标 |
| macOS | `assets/icon.icns` | App 包 `Contents/Resources/AppIcon.icns` + `CFBundleIconFile` |
| Linux | `assets/icon.png` | deb 装到 `/usr/share/icons/hicolor/*/apps/` |

`packager.toml` 里三个都要列进 `icons`：

```toml
icons = ["assets/icon.png", "assets/icon.ico", "assets/icon.icns"]
```

### 为什么 Windows 装完桌面图标是白板？

cargo-packager 的 NSIS 模板是这样建快捷方式的：

```nsis
CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
```

它**没有**给 `CreateShortcut` 传图标参数，所以桌面/开始菜单快捷方式的图标
完全取自 `apipost-rs.exe` 自己内嵌的图标资源。而 `cargo build` 出来的 exe
默认没有任何图标资源 → 快捷方式就是默认白板图标。

修法就是仓库里的 `build.rs`：

```rust
let mut res = tauri_winres::WindowsResource::new();
res.set_icon("assets/icon.ico");
res.compile()?;
```

只对 Windows 生效（`[target.'cfg(windows)'.build-dependencies]`），
Linux/macOS 构建不受影响。资源编译器用 MSVC 的 `rc.exe`（windows-latest 自带），
或 mingw 的 `windres`。

### 为什么 Linux 装完桌面/启动器没有应用图标？

deb 把图标装进 `hicolor` 主题目录，而这个主题**只认标准尺寸名**。
cargo-packager 按图片**自身的像素尺寸**决定装到哪个子目录，所以直接列
1024×1024 的母版会得到：

```
/usr/share/icons/hicolor/1024x1024/apps/apipost-rs.png   # 主题里没有这个尺寸
```

hicolor 的 `index.theme` 只列 16/22/24/32/36/48/64/72/96/128/192/256/384/512，
`1024x1024` 不在其中 → `.desktop` 里的 `Icon=apipost-rs` 解析不到任何文件 →
桌面图标和启动器里都是空白（窗口本身不受影响）。

修法：从母版缩出标准尺寸放到 `assets/icons/`，在 `packager.toml` 里逐个列出
（Linux 只出 deb，不存在 AppImage"只取一个图标"的顾虑）：

```toml
icons = [
    "assets/icons/512x512.png", "assets/icons/256x256.png", "assets/icons/128x128.png",
    "assets/icons/64x64.png", "assets/icons/48x48.png", "assets/icons/32x32.png",
    "assets/icons/16x16.png", "assets/icon.ico", "assets/icon.icns",
]
```

不用真装就能验证包里的布局：

```bash
dpkg-deb -c dist/apipost-rs_0.1.0_amd64.deb | grep -E "icons/|applications/"
# 期望看到 hicolor/48x48/apps/apipost-rs.png 这类标准尺寸目录
```

装完后刷新缓存，图标才会出现（GNOME 会缓存 desktop entry）：

```bash
sudo update-desktop-database
sudo gtk-update-icon-cache -f /usr/share/icons/hicolor
# 仍不生效就重新登录一次
```

> 顺带一条：`.desktop` 的 `Categories` 只能有一个**主分类**，写
> `Development;Utility;Network;` 会被 `desktop-file-validate` 报 hint，
> 并让应用在菜单里出现两次 —— 现在只保留 `Development;`。

> 再顺带一提：NSIS 安装向导里"创建桌面快捷方式"是**安装完成页的复选框**，
> 静默安装（`/S`）时会自动创建。

## 二、各平台产物

| 平台 | 产物 | 命令 |
| --- | --- | --- |
| Windows | `dist/ApiPost-Rs_0.1.0_x64-setup.exe`（NSIS） | `cargo packager -c packager.toml -f nsis --release` |
| macOS | `dist/ApiPost-Rs.app`、`dist/ApiPost-Rs_0.1.0_universal.dmg` | `cargo packager -c packager.toml -f app,dmg --release` |
| Linux | `dist/apipost-rs_0.1.0_amd64.deb` | `cargo packager -c packager.toml -f deb --release` |

`cargo-packager` 安装：`cargo install cargo-packager --locked --version ^0.11`。

## 三、macOS：通用二进制与签名

### 通用二进制（Intel + Apple Silicon）

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
mkdir -p target/universal-apple-darwin/release
lipo -create -output target/universal-apple-darwin/release/apipost-rs \
  target/aarch64-apple-darwin/release/apipost-rs \
  target/x86_64-apple-darwin/release/apipost-rs
cargo packager -c packager.toml -f app,dmg --target universal-apple-darwin --release
```

### 签名 + 公证（可选）

配置下面这些仓库 secrets 后，`.github/workflows/release.yml` 会自动签名并公证；
没配置就产出未签名 dmg：

`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、
`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`

未签名的 dmg 在用户机器上会被 Gatekeeper 拦（提示"已损坏"之类），
用户需要：右键 App → 打开；或执行

```bash
xattr -dr com.apple.quarantine /Applications/ApiPost-Rs.app
```

## 四、Windows 细节

* `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`：
  发布版不弹黑色控制台窗口，debug 版保留控制台方便看日志。
* `[nsis] install-mode = "currentUser"`：装到用户目录，不需要管理员权限。
* 安装后图标缓存在少数机器上可能没刷新，注销/重启一次即可（或删除
  `%LOCALAPPDATA%\IconCache.db` 后重启 explorer）。

## 五、发布流程

版本号只有一处真相：`Cargo.toml` 的 `version`。发版时它、`packager.toml` 的
`version`、以及 git tag 三者必须一致 —— CI 会拦（见下）。

```bash
# 1) 改版本号（两处）
#    Cargo.toml    version = "0.2.0"
#    packager.toml version = "0.2.0"
# 2) 提交并打 tag
git commit -am "chore: 发布 v0.2.0"
git tag v0.2.0 && git push origin v0.2.0
```

工作流会在三平台各自构建并打包装到该 tag 的 Release：

* Ubuntu：`deb` + 原始二进制
* Windows：`setup.exe`（NSIS）+ 原始 exe
* macOS：`.app` + `.dmg`（通用二进制）+ 原始可执行文件

工作流里两个前置校验：

1. `assets/icon.*` 三个图标文件是否齐全 —— 图标缺失直接失败，
   避免又打出"装完没图标"的包；
2. tag / `Cargo.toml` / `packager.toml` 版本号是否一致 —— 不一致直接失败。
   自动更新是按版本号比对的，这里错了用户要么一直被告知"有新版本"，
   要么永远收不到提示。

另外 tag 里带 `-` 的（如 `v0.2.0-beta.1`）会被标成 **pre-release**。
GitHub 的 `/releases/latest` 接口会自动跳过 pre-release 和 draft，
所以灰度版本不会被客户端当成正式更新推给用户。

## 六、应用内自动更新检查

设置在「设置面板 → 关于」区域：显示当前版本，打开设置面板时自动查一次
（10 分钟内不重复请求，也可手动点「检查更新」）。

* 检查地址：`https://api.github.com/repos/<owner>/<repo>/releases/latest`，
  其中 `<owner>/<repo>` 从 `Cargo.toml` 的 `repository` 字段解析，本地版本取自
  `CARGO_PKG_VERSION` —— 都不需要额外配置。
* 版本比较：主版本三元组比大小；三元组相同时只认"远端是正式版、本地是预发布"
  （`1.0.0 > 1.0.0-beta.1`）。版本号解析不出来时一律不报"有新版本"，避免误报。
* 网络失败、仓库还没有 Release（404）、API 限流都会退化成一条红色提示，不会 panic，
  界面照常用。检查走应用里配置的代理。
* 有新版本时给出「下载新版本」（按平台优先挑 `.deb` / `setup.exe` / `.dmg`）
  和「打开发布页」两个按钮，直接用系统浏览器打开。

相关代码在 `src/app/updater.rs`（纯逻辑都有单测）：

```bash
# 单元测试（版本比较、附件挑选、JSON 解析）
cargo test --bin apipost-rs updater

# 联网冒烟测试：真实请求 GitHub API，验证 reqwest/UA/解析链路
cargo test --bin apipost-rs -- --ignored live_check_hits_github
```

> 仓库还没发过 Release 时，检查结果是"没有找到 release"，这是预期行为 ——
> 推第一个 tag 之后就会变成正常的版本对比。
