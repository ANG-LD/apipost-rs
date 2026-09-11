//! 构建脚本：只在 Windows 上把图标 + 版本信息写进 exe。
//!
//! 为什么必须有这一步：cargo-packager 的 NSIS 模板创建快捷方式时用的是
//! `CreateShortcut "$DESKTOP\...\lnk" "$INSTDIR\apipost-rs.exe"`（不指定图标参数），
//! 也就是**桌面快捷方式的图标完全取自 exe 内嵌的图标资源**。Rust 默认不会往 exe 里
//! 写图标，所以不嵌图标的话，装完桌面上就是一个白板图标。
//!
//! 非 Windows 平台这个脚本什么都不做（依赖也只在 Windows 上编译）。

fn main() {
    // 图标变了要重新编译资源
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut res = tauri_winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "ApiPost-Rs");
        res.set("ProductName", "ApiPost-Rs");
        res.set("LegalCopyright", "Copyright 2025 apipost-rs team");
        res.set("OriginalFilename", "apipost-rs.exe");
        if let Err(e) = res.compile() {
            // rc.exe 缺失时给出可操作的提示，而不是一句无头无尾的 io error
            panic!(
                "嵌入 Windows 图标失败: {e}\n\
                 需要资源编译器：MSVC 工具链的 rc.exe（windows-latest 自带），\
                 或 mingw 的 windres。"
            );
        }
    }
}
