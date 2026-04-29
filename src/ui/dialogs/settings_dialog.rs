//! 设置对话框组件
//!
//! 提供语言切换和主题切换功能。

use crate::app::AppState;
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::dialog::{DialogHeader, DialogTitle};
use gpui_component::{Sizable, StyledExt, WindowExt};
use std::sync::{Arc, Mutex};

/// 语言配置
const LANGUAGES: &[(&str, &str)] = &[("zh-CN", "\u{4e2d}\u{6587}"), ("en-US", "English")];

/// 主题配置
const THEMES: &[(&str, &str)] = &[
    ("dark", "\u{6697}\u{8272}"),
    ("light", "\u{6d45}\u{8272}"),
    ("sepia", "\u{6de1}\u{9ec4}\u{8272}"),
    ("system", "\u{8ddf}\u{968f}\u{7cfb}\u{7edf}"),
];

/// 打开设置对话框
///
/// 在对话框中提供语言选择和主题选择功能。
/// 点击按钮后直接生效，无需保存。
pub fn open_settings_dialog(app_state: &AppState, window: &mut Window, cx: &mut App) {
    let current_lang = app_state.config.general.language.clone();
    let current_theme = app_state.config.general.theme.clone();
    let dialog_app_state = Arc::new(Mutex::new(app_state.clone()));
    let lang_for_ui = current_lang.clone();
    let theme_for_ui = current_theme.clone();
    let ui_theme = Theme::from_str(&current_theme);

    window.open_dialog(cx, move |dialog, _, _| {
        let ui_theme = ui_theme.clone();
        dialog
            .w(px(400.0))
            .title("\u{8bbe}\u{7f6e} / Settings")
            .content({
                let lang_for_ui = lang_for_ui.clone();
                let theme_for_ui = theme_for_ui.clone();
                let app_state_for_click = dialog_app_state.clone();
                move |content, _, _| {
                    content
                        .child(
                            DialogHeader::new()
                                .child(DialogTitle::new().child("\u{8bbe}\u{7f6e} / Settings")),
                        )
                        .child(
                            gpui::div()
                                .p_4()
                                .flex_col()
                                .gap_4()
                                .child(language_section(
                                    &lang_for_ui,
                                    app_state_for_click.clone(),
                                    &ui_theme,
                                ))
                                .child(theme_section(
                                    &theme_for_ui,
                                    app_state_for_click.clone(),
                                    &ui_theme,
                                )),
                        )
                }
            })
            .footer(
                gpui::div().flex().justify_end().child(
                    Button::new("close-settings")
                        .label("\u{5173}\u{95ed} / Close")
                        .on_click(|_, window, cx| {
                            window.close_dialog(cx);
                        }),
                ),
            )
    });
}

/// 语言选择区域
fn language_section(
    current_lang: &str,
    app_state: Arc<Mutex<AppState>>,
    theme: &Theme,
) -> impl IntoElement {
    gpui::div()
        .flex_col()
        .gap_2()
        .child(
            gpui::div()
                .text_sm()
                .font_semibold()
                .text_color(theme.muted_foreground)
                .child("\u{8bed}\u{8a00} / Language"),
        )
        .child(gpui::div().flex().gap_2().children(
            LANGUAGES.iter().map(move |(code, label)| {
                let is_active = current_lang == *code;
                let app_state = app_state.clone();
                let lang_code = code.to_string();
                Button::new(format!("lang-{}", code))
                    .label(label.to_string())
                    .flex_1()
                    .h(px(36.0))
                    .bg(if is_active {
                        theme.accent
                    } else {
                        theme.input_background
                    })
                    .text_color(rgb(0xffffff))
                    .on_click(move |_, _, _| {
                        if let Ok(mut s) = app_state.lock() {
                            s.switch_language(&lang_code);
                        }
                    })
            }),
        ))
}

/// 主题选择区域
fn theme_section(
    current_theme: &str,
    app_state: Arc<Mutex<AppState>>,
    theme: &Theme,
) -> impl IntoElement {
    gpui::div()
        .flex_col()
        .gap_2()
        .child(
            gpui::div()
                .text_sm()
                .font_semibold()
                .text_color(theme.muted_foreground)
                .child("\u{4e3b}\u{9898} / Theme"),
        )
        .child(gpui::div().flex().flex_wrap().gap_2().children(
            THEMES.iter().map(move |(code, label)| {
                let is_active = current_theme == *code;
                let app_state = app_state.clone();
                let theme_code = code.to_string();
                Button::new(format!("theme-{}", code))
                    .label(label.to_string())
                    .min_w(px(80.0))
                    .h(px(36.0))
                    .px_3()
                    .bg(if is_active {
                        theme.accent
                    } else {
                        theme.input_background
                    })
                    .text_color(rgb(0xffffff))
                    .on_click(move |_, _, _| {
                        if let Ok(mut s) = app_state.lock() {
                            s.set_theme(&theme_code);
                        }
                    })
            }),
        ))
}
