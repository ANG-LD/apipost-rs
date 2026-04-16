//! ApiPost-Rs 主程序入口
//!
//! 一个使用Rust和gpui框架构建的PostMan替代工具
//! 提供API测试、环境变量管理、历史记录等功能

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
    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("ApiPost-Rs 启动中...");

    // 加载应用配置
    let config = AppConfig::load().unwrap_or_default();
    info!("配置加载完成: 语言={}, 主题={}", config.general.language, config.general.theme);

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

        // 初始化应用状态
        let app_state = AppState::try_new(config.clone())
            .expect("应用初始化失败");

        app_state.init();

        // 启动主窗口
        let app_state = Arc::new(app_state);
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(
                    TitlebarOptions {
                        title: Some("ApiPost-Rs".into()),
                        ..Default::default()
                    }
                ),
                ..Default::default()
            },
            |window, cx| {
                let main_view = cx.new(|cx| MainView::new(app_state.clone(), window, cx));
                cx.new(|cx| Root::new(main_view, window, cx))
            },
        );

        info!("主窗口已创建");
    });
}
