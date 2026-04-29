//! 代码生成对话框组件
//!
//! 将当前请求转换为多种编程语言的代码片段，支持复制到剪贴板。

use crate::http::{generate_code, HttpRequest};
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::dialog::{DialogHeader, DialogTitle};
use gpui_component::input::{Input, InputState};
use gpui_component::{Sizable, StyledExt, WindowExt};

/// 支持的编程语言列表
const CODE_LANGUAGES: &[(&str, &str)] = &[
    ("curl", "cURL"),
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("go", "Go"),
    ("rust", "Rust"),
    ("java", "Java"),
    ("php", "PHP"),
];

/// 代码生成对话框状态
pub struct CodeGenDialogState {
    /// 当前选中的语言
    selected_language: String,
    /// 生成的代码内容
    generated_code: String,
    /// 代码显示输入框
    code_input: Option<Entity<InputState>>,
}

impl CodeGenDialogState {
    /// 创建新的代码生成状态，默认使用 cURL
    pub fn new(request: &HttpRequest) -> Self {
        let language = "curl".to_string();
        let code = generate_code(request, &language);
        Self {
            selected_language: language,
            generated_code: code,
            code_input: None,
        }
    }

    /// 切换语言并重新生成代码
    pub fn switch_language(&mut self, request: &HttpRequest, language: &str) {
        self.selected_language = language.to_string();
        self.generated_code = generate_code(request, language);
    }
}

/// 打开代码生成对话框
///
/// 根据当前请求生成多种编程语言的代码片段。
/// 用户可以选择语言并复制生成的代码。
pub fn open_code_gen_dialog(request: HttpRequest, window: &mut Window, cx: &mut App, theme: Theme) {
    let state = CodeGenDialogState::new(&request);

    let theme_for_content = theme.clone();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        dialog
            .w(px(680.0))
            .h(px(500.0))
            .title("Code Generation")
            .content({
                let request_for_ui = request.clone();
                let theme = theme_for_content.clone();
                move |content, _window, cx| {
                    content
                        .child(
                            DialogHeader::new()
                                .child(DialogTitle::new().child("Code Generation")),
                        )
                        .child(
                            gpui::div()
                                .p_4()
                                .flex_col()
                                .gap_3()
                                .child(code_language_selector(&request_for_ui, cx, &theme))
                                .child(code_display_area(&theme)),
                        )
                }
            })
            .footer(
                gpui::div().flex().justify_end().child(
                    Button::new("close-code-gen")
                        .label("Close")
                        .on_click(|_, window, cx| {
                            window.close_dialog(cx);
                        }),
                ),
            )
    });
}

/// 语言选择器区域
///
/// TODO: 当前为静态语言按钮，后续可改为响应式状态切换。
fn code_language_selector(request: &HttpRequest, cx: &mut App, theme: &Theme) -> impl IntoElement {
    let initial_code = generate_code(request, "curl");

    gpui::div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .children(CODE_LANGUAGES.iter().map(|(code, label)| {
            let lang = code.to_string();
            let req_clone = request.clone();
            Button::new(format!("code-lang-{}", code))
                .label(label.to_string())
                .min_w(px(80.0))
                .small()
                .px_3()
                .py_1()
                .rounded_sm()
                .text_sm()
                .bg(theme.input_background)
                .text_color(rgb(0xcccccc))
                .on_click(move |_, _window, _cx| {
                    let _code = generate_code(&req_clone, &lang);
                    // TODO: 更新代码显示区域内容
                })
        }))
}

/// 代码显示区域
fn code_display_area(theme: &Theme) -> impl IntoElement {
    gpui::div()
        .flex_1()
        .overflow_hidden()
        .bg(theme.background)
        .border_1()
        .border_color(theme.muted_background)
        .rounded_md()
        .p_3()
        .child(
            gpui::div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .font_family("monospace")
                .child("Select a language to generate code"),
        )
}
