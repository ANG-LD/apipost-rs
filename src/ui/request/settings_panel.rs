use crate::ui::main_view::MainView;
use crate::ui::settings::RequestSettings;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

pub fn render_settings_panel(
    this: &mut MainView,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let settings = this.settings.clone();
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);

    div()
        .flex_col()
        .flex_1()
        .gap_4()
        .p_3()
        .overflow_y_scrollbar()
        .children([
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .children([
                    div()
                        .w(px(140.0))
                        .text_sm()
                        .text_color(theme.foreground)
                        .child("Timeout (s):")
                        .into_any_element(),
                    div()
                        .h(px(32.0))
                        .w(px(80.0))
                        .bg(theme.input_background)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            Input::new(&this.settings_inputs.timeout_input)
                                .small()
                                .h(px(30.0))
                                .w(px(76.0))
                                .bg(theme.input_background)
                                .text_color(theme.foreground),
                        )
                        .into_any_element(),
                ])
                .into_any_element(),
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .children([
                    div()
                        .w(px(140.0))
                        .text_sm()
                        .text_color(theme.foreground)
                        .child("Retries:")
                        .into_any_element(),
                    div()
                        .h(px(32.0))
                        .w(px(80.0))
                        .bg(theme.input_background)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            Input::new(&this.settings_inputs.retry_input)
                                .small()
                                .h(px(30.0))
                                .w(px(76.0))
                                .bg(theme.input_background)
                                .text_color(theme.foreground),
                        )
                        .into_any_element(),
                ])
                .into_any_element(),
            toggle_row("Follow Redirects:", settings.follow_redirects, cx, &theme, |this, cx| {
                this.toggle_follow_redirects(cx);
            }),
            toggle_row("Verify SSL:", settings.verify_ssl, cx, &theme, |this, cx| {
                this.toggle_verify_ssl(cx);
            }),
        ])
        .into_any_element()
}

fn toggle_row(
    label: &'static str,
    value: bool,
    cx: &mut Context<MainView>,
    theme: &Theme,
    on_toggle: impl Fn(&mut MainView, &mut Context<MainView>) + 'static,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_3()
        .children([
            div()
                .w(px(140.0))
                .text_sm()
                .text_color(theme.foreground)
                .child(label)
                .into_any_element(),
            div()
                .text_color(if value {
                    theme.success
                } else {
                    theme.muted_foreground
                })
                .on_mouse_down(MouseButton::Left, cx.listener(
                    move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                        on_toggle(this, cx);
                    },
                ))
                .child(if value { "ON" } else { "OFF" })
                .into_any_element(),
        ])
        .into_any_element()
}
