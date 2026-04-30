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
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .overflow_hidden()
        .gap(px(8.0))
        .px_2()
        .py_2()
        .children([
            div()
                .h(px(34.0))
                .w(px(95.0))
                .flex()
                .child(
                    Select::new(&this.method_select)
                        .small()
                        .h(px(34.0))
                        .border_1()
                        .border_color(rgb(0x555555))
                        .rounded_sm()
                        .text_color(rgb(method_color(&method)))
                        .font_semibold()
                        .flex_none(),
                ),
            div()
                .flex_1()
                .w_full()
                .h(px(34.0))
                .flex()
                .child(
                    Input::new(&this.url_input)
                        .h(px(34.0))
                        .w_full()
                        .bg(theme.code_background)
                        .border_1()
                        .border_color(rgb(0x555555))
                        .rounded_sm()
                        .text_color(theme.foreground),
                ),
            div()
                .flex()
                .h(px(34.0))
                .w(px(95.0))
                .mr_1()
                .child(
                    Button::new("send")
                        .px_4()
                        .rounded_sm()
                        .bg(if is_loading {
                            theme.muted_foreground
                        } else {
                            theme.accent
                        })
                        .text_color(rgb(0xffffff))
                        .font_semibold()
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
                                },
                            ),
                        ),
                ),
            div()
                .flex()
                .h(px(34.0))
                .mr_2()
                .child(
                    Button::new("save-request")
                        .h(px(34.0))
                        .px_3()
                        .rounded_sm()
                        .bg(theme.input_background)
                        .text_color(theme.foreground)
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
