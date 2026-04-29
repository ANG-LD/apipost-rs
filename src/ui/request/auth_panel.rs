use crate::ui::authorization::{ApiKeyLocation, AuthState, AuthType};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::Sizable;
use gpui_component::StyledExt;

pub fn render_auth_panel(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let auth_type = this.get_auth_type();
    let auth_state = this.auth_state.clone();

    div()
        .flex_col()
        .flex_1()
        .gap_4()
        .p_3()
        .overflow_y_hidden()
        .children([
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .children([
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Type:"),
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .children([
                            auth_type_button(
                                this,
                                AuthType::NoAuth,
                                auth_type,
                                this.t("ui.no_auth"),
                                window,
                                cx,
                            ),
                            auth_type_button(
                                this,
                                AuthType::BearerToken,
                                auth_type,
                                this.t("ui.bearer_token"),
                                window,
                                cx,
                            ),
                            auth_type_button(
                                this,
                                AuthType::BasicAuth,
                                auth_type,
                                this.t("ui.basic_auth"),
                                window,
                                cx,
                            ),
                            auth_type_button(
                                this,
                                AuthType::ApiKey,
                                auth_type,
                                this.t("ui.api_key"),
                                window,
                                cx,
                            ),
                        ]),
                ]),
            match &auth_state {
                AuthState::NoAuth => div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("This request does not use any authorization."),
                AuthState::Bearer(auth) => div()
                    .flex_col()
                    .gap_3()
                    .flex_1()
                    .children([
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Token"),
                        div().child(
                            Input::new(&auth.token)
                                .small()
                                .h(px(32.0))
                                .w(px(400.0))
                                .bg(theme.code_background)
                                .border_1()
                                .border_color(theme.border)
                                .text_color(theme.foreground),
                        ),
                    ]),
                AuthState::Basic(auth) => div()
                    .flex_col()
                    .gap_3()
                    .flex_1()
                    .children([
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Username"),
                        div().child(
                            Input::new(&auth.username)
                                .small()
                                .h(px(32.0))
                                .w(px(400.0))
                                .bg(theme.code_background)
                                .border_1()
                                .border_color(theme.border)
                                .text_color(theme.foreground),
                        ),
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Password"),
                        div().child(
                            Input::new(&auth.password)
                                .small()
                                .h(px(32.0))
                                .w(px(400.0))
                                .bg(theme.code_background)
                                .border_1()
                                .border_color(theme.border)
                                .text_color(theme.foreground),
                        ),
                    ]),
                AuthState::ApiKey(auth) => {
                    let location_value = auth.location_value;
                    div()
                        .flex_col()
                        .gap_3()
                        .flex_1()
                        .children([
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Key"),
                            div().child(
                                Input::new(&auth.key)
                                    .small()
                                    .h(px(32.0))
                                    .w(px(400.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Value"),
                            div().child(
                                Input::new(&auth.value)
                                    .small()
                                    .h(px(32.0))
                                    .w(px(400.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Add to"),
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .children([
                                    div()
                                        .text_sm()
                                        .cursor_pointer()
                                        .min_w(px(70.0))
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .bg(
                                            if location_value
                                                == ApiKeyLocation::Header
                                            {
                                                theme.muted_background
                                            } else {
                                                theme.code_background
                                            },
                                        )
                                        .text_color(
                                            if location_value
                                                == ApiKeyLocation::Header
                                            {
                                                rgb(0xffffff)
                                            } else {
                                                theme.muted_foreground
                                            },
                                        )
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                |this,
                                                 _: &MouseDownEvent,
                                                 _window: &mut Window,
                                                 cx: &mut Context<
                                                    MainView,
                                                >| {
                                                    this.toggle_api_key_location(cx);
                                                },
                                            ),
                                        )
                                        .child(this.t("ui.header")),
                                    div()
                                        .text_sm()
                                        .cursor_pointer()
                                        .min_w(px(70.0))
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .bg(
                                            if location_value
                                                == ApiKeyLocation::Query
                                            {
                                                theme.muted_background
                                            } else {
                                                theme.code_background
                                            },
                                        )
                                        .text_color(
                                            if location_value
                                                == ApiKeyLocation::Query
                                            {
                                                rgb(0xffffff)
                                            } else {
                                                theme.muted_foreground
                                            },
                                        )
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                |this,
                                                 _: &MouseDownEvent,
                                                 _window: &mut Window,
                                                 cx: &mut Context<
                                                    MainView,
                                                >| {
                                                    this.toggle_api_key_location(cx);
                                                },
                                            ),
                                        )
                                        .child(this.t("ui.query")),
                                ]),
                        ])
                }
            },
        ])
}

fn auth_type_button(
    this: &mut MainView,
    auth_type: AuthType,
    current: AuthType,
    label: String,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let is_active = current == auth_type;
    div()
        .text_sm()
        .cursor_pointer()
        .min_w(px(80.0))
        .px_2()
        .py_1()
        .rounded_sm()
        .bg(if is_active {
            theme.muted_background
        } else {
            theme.code_background
        })
        .text_color(if is_active {
            rgb(0xffffff)
        } else {
            theme.muted_foreground
        })
        .on_mouse_down(MouseButton::Left, cx.listener(
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.set_auth_type(auth_type.to_index(), window, cx);
            },
        ))
        .child(label)
}
