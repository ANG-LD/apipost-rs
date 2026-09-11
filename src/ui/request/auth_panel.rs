use crate::ui::authorization::{ApiKeyLocation, AuthState, AuthType};
use crate::ui::components::{
    form_row, section_divider, segment_button, segment_group, CONTROL_H, GAP_M, GAP_S, GAP_XS,
    PANEL_PAD,
};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

/// 认证面板：认证类型分段按钮 + 对应字段表单
///
/// 之前这里的字段标签是写死的英文（Token / Username / Password / Key / Value / Add to），
/// 切换语言不会变；类型按钮与 Header/Query 按钮也各写一套样式（深色主题下选中/未选中同色）。
/// 现在标签全部走 i18n，按钮统一用 segment_button，字段用统一的 form_row 排布。
pub fn render_auth_panel(
    this: &mut MainView,
    _window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = this.cached_theme.clone();
    let auth_type = this.get_auth_type();
    let auth_state = this.auth_state.clone();

    let t_type = this.t("auth.type");
    let t_effective = this.t("auth.effective");
    // 当前认证实际会带到请求上的内容，给用户一个直观确认
    let effective = this.auth_state.to_headers(cx);

    // 生效内容预览：直接显示这次请求会带上的认证信息
    // （overflow_y_scrollbar() 返回 Scrollable，不能再链 .when，所以先算好再 children）
    let preview: Option<AnyElement> = if effective.is_empty() {
        None
    } else {
        Some(
            div()
                .flex_col()
                .gap(px(GAP_M))
                .child(section_divider(&theme))
                .child(form_row(
                    t_effective.clone(),
                    &theme,
                    div()
                        .flex_col()
                        .gap(px(GAP_XS))
                        .children(effective.iter().map(|(key, value)| {
                            div()
                                .px(px(GAP_S))
                                .py(px(GAP_XS))
                                .rounded_md()
                                .bg(theme.muted_background)
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{}: {}", key, shorten(value, 48)))
                                .into_any_element()
                        }))
                        .into_any_element(),
                ))
                .into_any_element(),
        )
    };

    div()
        .id("auth-panel")
        .flex_col()
        .flex_1()
        .gap(px(GAP_M))
        .p(px(PANEL_PAD))
        .overflow_y_scrollbar()
        .child(form_row(
            t_type,
            &theme,
            // items_start：分段组是内容宽度，不要被表单行拉满
            div()
                .flex()
                .flex_row()
                .items_start()
                .child(
                    segment_group(&theme).children([
                        auth_type_button(
                            this,
                            AuthType::NoAuth,
                            auth_type,
                            this.t("ui.no_auth"),
                            cx,
                        )
                        .into_any_element(),
                        auth_type_button(
                            this,
                            AuthType::BearerToken,
                            auth_type,
                            this.t("ui.bearer_token"),
                            cx,
                        )
                        .into_any_element(),
                        auth_type_button(
                            this,
                            AuthType::BasicAuth,
                            auth_type,
                            this.t("ui.basic_auth"),
                            cx,
                        )
                        .into_any_element(),
                        auth_type_button(
                            this,
                            AuthType::ApiKey,
                            auth_type,
                            this.t("ui.api_key"),
                            cx,
                        )
                        .into_any_element(),
                    ]),
                )
                .into_any_element(),
        ))
        .child(section_divider(&theme))
        .child(match &auth_state {
            AuthState::NoAuth => div()
                .py(px(GAP_M))
                .child(hint(this.t("auth.none_hint"), &theme))
                .into_any_element(),
            AuthState::Bearer(auth) => div()
                .flex_col()
                .gap(px(GAP_S))
                .child(hint(this.t("auth.bearer_hint"), &theme))
                .child(field_row(this.t("auth.token"), &auth.token, &theme))
                .into_any_element(),
            AuthState::Basic(auth) => div()
                .flex_col()
                .gap(px(GAP_S))
                .child(hint(this.t("auth.basic_hint"), &theme))
                .child(field_row(this.t("auth.username"), &auth.username, &theme))
                .child(field_row(this.t("auth.password"), &auth.password, &theme))
                .into_any_element(),
            AuthState::ApiKey(auth) => {
                let location_value = auth.location_value;
                div()
                    .flex_col()
                    .gap(px(GAP_S))
                    .child(hint(this.t("auth.apikey_hint"), &theme))
                    .child(field_row(this.t("auth.key"), &auth.key, &theme))
                    .child(field_row(this.t("auth.value"), &auth.value, &theme))
                    .child(form_row(
                        this.t("auth.add_to"),
                        &theme,
                        div()
                            .flex()
                            .flex_row()
                            .items_start()
                            .child(
                                segment_group(&theme).children([
                                    segment_button(
                                        "apikey-loc-header",
                                        this.t("ui.header"),
                                        location_value == ApiKeyLocation::Header,
                                        &theme,
                                    )
                                    .on_mouse_down(MouseButton::Left, cx.listener(
                                        |this, _: &MouseDownEvent, _w: &mut Window, cx: &mut Context<MainView>| {
                                            this.set_api_key_location(ApiKeyLocation::Header, cx);
                                        },
                                    ))
                                    .into_any_element(),
                                    segment_button(
                                        "apikey-loc-query",
                                        this.t("ui.query"),
                                        location_value == ApiKeyLocation::Query,
                                        &theme,
                                    )
                                    .on_mouse_down(MouseButton::Left, cx.listener(
                                        |this, _: &MouseDownEvent, _w: &mut Window, cx: &mut Context<MainView>| {
                                            this.set_api_key_location(ApiKeyLocation::Query, cx);
                                        },
                                    ))
                                    .into_any_element(),
                                ]),
                            )
                            .into_any_element(),
                    ))
                    .into_any_element()
            }
        })
        .children(preview)
}

/// 字段行：标签 + 输入框（与设置面板同一套样式与列宽）
fn field_row(label: impl Into<SharedString>, state: &Entity<InputState>, theme: &Theme) -> AnyElement {
    let label: SharedString = label.into();
    form_row(
        label,
        theme,
        Input::new(state)
            .h(px(CONTROL_H - 2.0))
            .w(px(420.0))
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.input_background)
            .text_color(theme.foreground)
            .into_any_element(),
    )
}

/// 弱化提示文字
fn hint(text: impl Into<SharedString>, theme: &Theme) -> AnyElement {
    let text: SharedString = text.into();
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text)
        .into_any_element()
}

/// 过长的 token/JWT 截断显示，避免把面板撑宽
fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let mut out: String = value.chars().take(max).collect();
        out.push('…');
        out
    }
}

/// 认证类型按钮：与 Body 类型、响应视图共用同一套分段按钮
fn auth_type_button(
    this: &mut MainView,
    auth_type: AuthType,
    current: AuthType,
    label: impl Into<SharedString>,
    cx: &mut Context<MainView>,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let theme = this.cached_theme.clone();
    segment_button(
        format!("auth-type-{}", auth_type.to_index()),
        label,
        current == auth_type,
        &theme,
    )
    .on_click(cx.listener(
        move |this, _: &ClickEvent, window: &mut Window, cx: &mut Context<MainView>| {
            this.set_auth_type(auth_type.to_index(), window, cx);
        },
    ))
}
