//! 应用自己的资源源（`AssetSource` 桥接）
//!
//! 为什么必须有这一层（不写就永远看不到自绘图标）：
//! `gpui::svg()` 画图标时不读文件系统，而是把 `path` 交给**全局资源源**去查
//! （gpui `window.rs` 的 `Window::paint_svg` → `SvgRenderer::render_alpha_mask`
//! 里那句 `self.asset_source.load(&params.path)`，返回 None/Err 就整块不画）。
//! 本项目原来装的是 `gpui_component_assets::Assets`：它用 RustEmbed 只把组件库
//! 自己那份 `assets/icons/**/*.svg` 打进二进制，只认 `icons/<组件库图标名>.svg`
//! 这一组路径。应用自己的 `assets/icons/history.svg` 谁都解析不到 ——
//! 文件躺在仓库里、界面却空白，原因就在这里。
//!
//! 桥接的规则只有一条：**先认自己的图标（编译期内嵌），其余全部原样委托给
//! 组件库资源源**。委托那条路不能漏：组件库图标和字体都靠它，漏了整个界面的
//! 图标会一起变空。
//!
//! 刻意不引入 `rust-embed` 之类新依赖：这里只需要一个文件，
//! `include_str!` 就是标准库自带的"把文件嵌进二进制"。

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// 自绘「历史记录」图标在资源源里的路径。
///
/// 前缀沿用组件库的 `icons/`，并且与磁盘位置 `assets/icons/history.svg` 一一对应 ——
/// 调用点写出来的路径和仓库目录不必记两套命名规则。
/// 组件库当前没有同名图标；万一将来加了，本应用的先命中（见下面的 `load`）。
pub const HISTORY_ICON_PATH: &str = "icons/history.svg";

/// 本应用实际安装的资源源 = 应用自绘图标 + 组件库全部资源
pub struct Assets {
    /// 组件库资源源：图标与字体都由它提供，除上面命中的路径外一律走它
    inner: gpui_component_assets::Assets,
}

impl Assets {
    pub fn new() -> Self {
        // 用 `new("")` 而不是 `default()`：native 下组件库的 `Assets` 是单元结构体
        // （没有 `Default` 实现），wasm 下是带 endpoint 的结构体 —— `new` 两个平台都有。
        // native 下 endpoint 参数按定义被忽略，这里传空串没有副作用。
        Self {
            inner: gpui_component_assets::Assets::new(""),
        }
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // 先命中应用自己的图标：`include_str!` 在编译期把 SVG 文本嵌进二进制，
        // 运行时不依赖工作目录或安装路径，`cargo run` 与装好之后行为一致。
        // `as_bytes()` 借用的是 `'static` 字面量，所以可以直接 `Cow::Borrowed`，零拷贝。
        if path == HISTORY_ICON_PATH {
            return Ok(Some(Cow::Borrowed(
                include_str!("../assets/icons/history.svg").as_bytes(),
            )));
        }

        // 其余全部委托：组件库图标（`icons/*.svg`）与字体（`fonts/**`）都靠这一句
        self.inner.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut entries = self.inner.list(path)?;
        // `list` 用于列举/热重载。自绘图标也要报出去，否则"资源源声称拥有的集合"
        // 和"实际能 load 的集合"对不上，按清单检查资产的地方会误判它不存在。
        if HISTORY_ICON_PATH.starts_with(path) {
            entries.push(SharedString::from(HISTORY_ICON_PATH));
        }
        Ok(entries)
    }
}
