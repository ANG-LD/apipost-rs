use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

pub fn render_pre_request_panel(
    this: &mut MainView,
    _window: &mut Window,
    _cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .p_3()
        .overflow_y_scrollbar()
        .children([
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(
                    "Pre-request Script (JavaScript) - Runs before the request is sent",
                ),
            div()
                .flex_1()
                .bg(theme.muted_background)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .overflow_hidden()
                .child(
                    Input::new(&this.script_state.pre_request_script)
                        .flex_1()
                        .min_h(px(200.0))
                        .bg(theme.muted_background)
                        .text_color(theme.foreground)
                        .font_family("monospace"),
                ),
        ])
}

pub fn render_tests_panel(
    this: &mut MainView,
    _window: &mut Window,
    _cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .p_3()
        .overflow_y_scrollbar()
        .children([
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(
                    "Test Script (JavaScript) - Runs after the response is received",
                ),
            div()
                .flex_1()
                .bg(theme.muted_background)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .overflow_hidden()
                .child(
                    Input::new(&this.script_state.test_script)
                        .flex_1()
                        .min_h(px(200.0))
                        .bg(theme.muted_background)
                        .text_color(theme.foreground)
                        .font_family("monospace"),
                ),
        ])
}
