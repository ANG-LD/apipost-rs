use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::{IconName, Sizable, StyledExt};

pub fn render_headers_panel(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let headers = this.headers.clone();

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .p_3()
        .overflow_y_hidden()
        .children([
            div()
                .flex()
                .flex_row()
                .gap_2()
                .mb_1()
                .children([
                    div()
                        .w(px(30.0))
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(""),
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(this.t("ui.key")),
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(this.t("ui.value")),
                    div()
                        .w(px(30.0))
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(""),
                ]),
            div()
                .flex_col()
                .gap_2()
                .py_1()
                .mt_1()
                .children(headers.iter().enumerate().map(|(idx, header)| {
                    div()
                        .mt_1()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .children([
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(if header.enabled {
                                    theme.success
                                } else {
                                    theme.muted_foreground
                                })
                                .child(if header.enabled { "✓" } else { "○" }),
                            div().flex_1().child(
                                Input::new(&header.key)
                                    .small()
                                    .h(px(32.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            div().flex_1().child(
                                Input::new(&header.value)
                                    .small()
                                    .h(px(32.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            div()
                                .w(px(20.0))
                                .h(px(20.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Button::new(idx.to_string())
                                        .small()
                                        .icon(IconName::Close)
                                        .text_color(theme.muted_foreground)
                                        .bg(theme.background)
                                        .on_click(cx.listener(
                                            move |this,
                                                  _: &ClickEvent,
                                                  _window: &mut Window,
                                                  cx: &mut Context<MainView>| {
                                                this.remove_header(idx, cx);
                                            },
                                        )),
                                ),
                        ])
                })),
            div().mt_2().px_1().py_1().child(
                Button::new("add-header")
                    .min_w(px(100.0))
                    .px_2()
                    .py_1()
                    .text_sm()
                    .icon(IconName::Plus)
                    .text_color(theme.accent)
                    .bg(theme.background)
                    .label(this.t("ui.add_header"))
                    .on_click(cx.listener(
                        |this: &mut MainView,
                         _: &ClickEvent,
                         window: &mut Window,
                         cx: &mut Context<MainView>| {
                            this.add_header(window, cx);
                        },
                    )),
            ),
        ])
}
