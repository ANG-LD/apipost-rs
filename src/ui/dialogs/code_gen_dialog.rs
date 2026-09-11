//! 代码生成对话框
//!
//! 将当前请求转换为多种编程语言的代码片段，支持复制到剪贴板。

use crate::http::{generate_code, HttpRequest};
use crate::ui::clipboard;
use crate::ui::json_editor::code_editor_view;
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::InputState;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use std::sync::{Arc, Mutex};

const CODE_LANGUAGES: &[(&str, &str)] = &[
    ("curl", "cURL"),
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("go", "Go"),
    ("rust", "Rust"),
    ("java", "Java"),
    ("php", "PHP"),
];

pub struct CodeGenDialogState {
    pub open: bool,
    pub request: Option<HttpRequest>,
    pub selected_language: String,
    pub generated_code: String,
    pub code_input: Option<Entity<InputState>>,
    pub toast_message: Option<String>,
    needs_update: bool,
}

impl CodeGenDialogState {
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        let code_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("")
                .multi_line(true)
        });
        Self {
            open: false,
            request: None,
            selected_language: "curl".to_string(),
            generated_code: String::new(),
            code_input: Some(code_input),
            toast_message: None,
            needs_update: false,
        }
    }

    pub fn open_dialog(&mut self, request: HttpRequest) {
        self.request = Some(request.clone());
        self.selected_language = "curl".to_string();
        self.generated_code = generate_code(&request, "curl");
        self.open = true;
        self.toast_message = None;
        // 标记需要同步到 InputState（render 中执行）
        self.needs_update = true;
    }

    pub fn switch_language(&mut self, language: &str) {
        self.selected_language = language.to_string();
        if let Some(req) = &self.request {
            self.generated_code = generate_code(req, language);
        }
        self.needs_update = true;
    }

    pub fn copy_to_clipboard(&mut self) {
        if clipboard::copy_to_clipboard(&self.generated_code) {
            self.toast_message = Some("code.copied".to_string());
        } else {
            self.toast_message = Some("clipboard.copy_failed".to_string());
        }
    }
}

pub fn render_code_gen_dialog_overlay(
    state: &Arc<Mutex<CodeGenDialogState>>,
    theme: &Theme,
    entity_id: EntityId,
    t: &dyn Fn(&str) -> SharedString,
    window: &mut Window,
    cx: &mut Context<crate::ui::MainView>,
) -> impl IntoElement {
    let mut state_lock = state.lock().unwrap();
    if !state_lock.open {
        return gpui::div();
    }

    let selected = state_lock.selected_language.clone();
    let toast = state_lock.toast_message.clone();
    let code_input = state_lock.code_input.clone();

    // 仅在内容变化时同步到 InputState（multi_line 模式支持换行）
    if state_lock.needs_update {
        let code = state_lock.generated_code.clone();
        if let Some(ref input) = state_lock.code_input {
            input.update(cx, |input, cx| {
                input.set_value(&code, window, cx);
            });
        }
        state_lock.needs_update = false;
    }
    drop(state_lock);

    let toast_text = toast.as_ref().map(|key| t(key));
    let state_rc = Arc::clone(state);
    let theme_rc = theme.clone();

    gpui::div()
        .absolute()
        .inset_0()
        .bg(rgba(0x00000088))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _window: &mut Window, cx: &mut App| cx.stop_propagation())
        .child(
            gpui::div()
                .w(px(660.0))
                .h(px(520.0))
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_2xl()
                .flex()
                .flex_col()
                .child(
                    // 标题栏
                    gpui::div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            gpui::div()
                                .text_lg()
                                .font_weight(FontWeight(600.0))
                                .text_color(theme.foreground)
                                .child(t("code.title")),
                        )
                        .child(
                            gpui::div()
                                .cursor_pointer()
                                .p_1()
                                .rounded_md()
                                .hover(|s| s.bg(theme.muted_background))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        let s = Arc::clone(&state_rc);
                                        move |this, _: &MouseDownEvent, _window, cx| {
                                            this.code_gen_dialog_state.lock().unwrap().open = false;
                                            cx.notify();
                                        }
                                    }),
                                )
                                .child(
                                    Icon::new(IconName::PanelLeftClose)
                                        .small()
                                        .text_color(theme.muted_foreground),
                                ),
                        ),
                )
                .child(
                    // 语言选择器
                    gpui::div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .px_4()
                        .py_3()
                        .children(CODE_LANGUAGES.iter().map(|(lang, label)| {
                            let lang_s = lang.to_string();
                            let is_active = selected == lang_s;
                            let state = Arc::clone(&state_rc);
                            let theme_btn = theme_rc.clone();
                            let eid = entity_id;
                            Button::new(format!("lang-{}", lang))
                                .label(label.to_string())
                                .min_w(px(72.0))
                                .small()
                                .when(is_active, |b| {
                                    b.bg(theme_btn.accent).text_color(theme_btn.accent_foreground)
                                })
                                .when(!is_active, |b| {
                                    b.bg(theme_btn.input_background).text_color(theme_btn.muted_foreground)
                                })
                                .on_click(move |_, _, cx| {
                                    state.lock().unwrap().switch_language(&lang_s);
                                    cx.notify(eid);
                                })
                        })),
                )
                .child({
                    let bg = theme.background;
                    gpui::div()
                        .flex_1()
                        .px_4()
                        .pb_3()
                        .min_h(px(200.0))
                        .when_some(code_input, move |d, input| {
                            d.child(code_editor_view(&input, bg, theme.foreground))
                        })
                })
                .child(
                    // Footer: toast + buttons
                    gpui::div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_3()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            gpui::div()
                                .text_sm()
                                .text_color(theme.success)
                                .when_some(toast_text, |d, msg| d.child(msg)),
                        )
                        .child(
                            gpui::div()
                                .flex()
                                .gap_2()
                                .child({
                                    let s = Arc::clone(&state_rc);
                                    let eid = entity_id;
                                    let label_copy = t("code.copy_btn");
                                    let eid = entity_id;
                                    Button::new("copy-code")
                                        .label(label_copy)
                                        .small()
                                        .on_click(move |_, _, cx| {
                                            s.lock().unwrap().copy_to_clipboard();
                                            cx.notify(eid);
                                        })
                                })
                                .child({
                                    let s = Arc::clone(&state_rc);
                                    let label_close = t("code.close_btn");
                                    let eid = entity_id;
                                    Button::new("close-code-gen")
                                        .label(label_close)
                                        .small()
                                        .on_click(move |_, _, cx| {
                                            s.lock().unwrap().open = false;
                                            cx.notify(eid);
                                        })
                                }),
                        ),
                ),
        )
}
