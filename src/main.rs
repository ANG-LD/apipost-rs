//! ApiPost-Rs 主程序入口
//!
//! 一个使用Rust和gpui框架构建的PostMan替代工具
//! 提供API测试、环境变量管理、历史记录等功能

// Windows 发布版不弹控制台窗口；debug 构建保留控制台，方便 cargo run 时看日志
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod http;
mod ui;
mod i18n;

use app::AppState;
use config::AppConfig;
use gpui::*;
use gpui_component::{Root, StyledExt};
use gpui_component_assets::Assets;
use gpui_platform::application;
use ui::MainView;
use log::info;
use std::sync::Arc;

fn main() {
    // 初始化日志系统（同时输出到 stderr 和文件）
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("apipost-rs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("debug.log");
    let log_file = std::io::BufWriter::new(
        std::fs::File::create(&log_path).expect("无法创建日志文件")
    );

    struct TeeWriter {
        file: std::sync::Mutex<std::io::BufWriter<std::fs::File>>,
    }
    impl std::io::Write for TeeWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let n = std::io::stderr().write(buf)?;
            self.file.lock().unwrap().write_all(buf)?;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            std::io::stderr().flush()?;
            self.file.lock().unwrap().flush()
        }
    }

    let tee = TeeWriter { file: std::sync::Mutex::new(log_file) };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        // 默认的 format_timestamp_millis 打的是 UTC（形如 08:53:56.506Z），
        // 和界面/数据库里的本地时间对不上，这里改成带时区偏移的本地时间
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {:5} {}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(tee)))
        .init();

    info!("ApiPost-Rs 启动中, 日志文件: {:?}", log_path);

    // 创建持久 tokio runtime（leak 到 'static，保证连接池存活至进程退出）
    // 注意：不能依赖 application().run() 闭包的生命周期 — 闭包在初始化后即被丢弃，
    // 但 event loop 仍在运行。通过 Box::leak 使 runtime 达到 'static 生命周期。
    let rt: &'static tokio::runtime::Runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("创建 tokio runtime 失败")
    ));
    let rt_handle = rt.handle().clone();
    info!("tokio runtime 已创建 (leaked to static), worker_threads=4");

    // 加载应用配置
    let config = AppConfig::load().unwrap_or_default();
    info!("配置加载完成: 语言={}, 主题={}", config.general.language, config.general.theme);
    // 使用 Arc 包裹配置，使 AppState::clone() 仅增加引用计数
    let config = Arc::new(config);

    // 构建并运行应用
    application().with_assets(Assets).run(move |cx: &mut App| {
        // 初始化gpui-component
        gpui_component::init(cx);

        // 根据配置初始化深色/浅色主题（影响语法高亮颜色）
        let theme_mode = if config.general.theme == "light" {
            gpui_component::theme::ThemeMode::Light
        } else {
            gpui_component::theme::ThemeMode::Dark
        };
        gpui_component::theme::Theme::change(theme_mode, None, cx);

        // 初始化应用状态（传入持久 runtime handle，config 所有权移入 AppState）
        let app_state = AppState::try_new(config, rt_handle)
            .expect("应用初始化失败");

        app_state.init();

        // 启动主窗口
        let app_state = Arc::new(std::sync::Mutex::new(app_state));
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(
                    TitlebarOptions {
                        title: Some("ApiPost-Rs".into()),
                        ..Default::default()
                    }
                ),
                app_id: Some("apipost-rs".into()),
                ..Default::default()
            },
            |window, cx| {
                let main_view = cx.new(|cx| MainView::new(app_state.clone(), window, cx));
                let weak_view = main_view.downgrade();
                main_view.update(cx, |view, cx| {
                    view.load_workspace(window, cx);
                    view.update_auto_save_task(window, cx);
                });

                // 窗口关闭时保存 workspace
                let _ = cx.on_window_closed(move |_app, _| {
                    if let Some(view) = weak_view.upgrade() {
                        view.update(_app, |view, cx| {
                            view.save_workspace(cx);
                        });
                    }
                    log::info!("应用已退出");
                });
                let root = cx.new(|cx| Root::new(main_view, window, cx));

                root
            },
        );

        info!("主窗口已创建");
    });
}
