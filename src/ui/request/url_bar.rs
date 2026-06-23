use crate::ui::components::method_color;
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::select::Select;
use gpui_component::{IconName, Sizable, StyledExt};

pub fn render_url_bar(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let is_loading = this.is_loading;
    let method = this.method.clone();

    div()
        .flex_none()
        .flex_shrink_0()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .overflow_hidden()
        .gap(px(6.0))
        .px_2()
        .py_1p5()
        .bg(theme.background)
        .border_b(px(1.0))
        .border_color(theme.border)
        .children([
            div()
                .h(px(32.0))
                .w(px(90.0))
                .flex()
                .child(
                    Select::new(&this.method_select)
                        .small()
                        .h(px(32.0))
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .text_color(rgb(method_color(&method)))
                        .font_semibold()
                        .flex_none(),
                ),
            div()
                .flex_1()
                .w_full()
                .h(px(32.0))
                .flex()
                .child(
                    Input::new(&this.url_input)
                        .h(px(32.0))
                        .w_full()
                        .bg(theme.input_background)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .text_sm()
                        .text_color(theme.foreground),
                ),
            div()
                .flex()
                .h(px(32.0))
                .w(px(88.0))
                .child(
                    Button::new("send")
                        .px_3()
                        .rounded_md()
                        .bg(if is_loading {
                            theme.muted_foreground
                        } else {
                            theme.accent
                        })
                        .text_color(rgb(0xffffff))
                        .font_semibold()
                        .text_xs()
                        .shadow_sm()
                        .icon(if is_loading {
                            IconName::LoaderCircle
                        } else {
                            IconName::Play
                        })
                        .label(if is_loading {
                            this.t("ui.sending")
                        } else {
                            this.t("ui.send")
                        })
                        .flex_none()
                        .on_click(
                            cx.listener(
                                |this: &mut MainView,
                                 _: &gpui::ClickEvent,
                                 window: &mut Window,
                                 cx: &mut Context<MainView>| {
                                    let url = this
                                        .url_input
                                        .read(cx)
                                        .value()
                                        .to_string();
                                    if url.trim().is_empty() {
                                        return;
                                    }
                                    let method = this
                                        .method_select
                                        .read(cx)
                                        .selected_value()
                                        .unwrap_or(&gpui::SharedString::from("GET"))
                                        .clone();
                                    this.url = url;
                                    this.method = method.to_string();
                                    this.send_request(window, cx);
                                    cx.notify();
                                },
                            ),
                        ),
                ),
            div()
                .flex()
                .h(px(32.0))
                .child(
                    Button::new("save-request")
                        .h(px(32.0))
                        .px_3()
                        .rounded_md()
                        .bg(theme.muted_background)
                        .text_color(theme.muted_foreground)
                        .text_xs()
                        .label(this.t("button.save"))
                        .on_click(cx.listener(
                            |this: &mut MainView,
                             _: &gpui::ClickEvent,
                             window: &mut Window,
                             cx: &mut Context<MainView>| {
                                this.save_current_request(window, cx);
                            },
                        )),
                ),
        ])
}
