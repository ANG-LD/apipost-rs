use crate::ui::components::{
    form_row, section_divider, toggle_switch, CONTROL_H, GAP_M, GAP_S, PANEL_PAD, RADIUS_SM,
};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

/// 请求设置面板：超时时间 / 重试次数 / 跟随重定向 / 校验 SSL
///
/// 之前这里的标签是写死的英文（Timeout (s): / Retries: ...），切换语言时不会变，
/// 且开关是 "ON"/"OFF" 文本。现在标签全部走 i18n，开关用统一组件。
pub fn render_settings_panel(
    this: &mut MainView,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let settings = this.settings.clone();
    let theme = this.cached_theme.clone();

    let t_timeout = this.t("settings.timeout");
    let t_retries = this.t("settings.retries");
    let t_follow = this.t("settings.follow_redirects");
    let t_verify = this.t("settings.verify_ssl");
    let t_sec = this.t("settings.unit_seconds");
    let t_times = this.t("settings.unit_times");

    // 先把输入框状态取出来，避免闭包里再次借用 this
    let timeout_input = this.settings_inputs.timeout_input.clone();
    let retry_input = this.settings_inputs.retry_input.clone();

    div()
        .id("request-settings")
        .flex_col()
        .flex_1()
        .gap(px(GAP_M))
        .p(px(PANEL_PAD))
        .overflow_y_scrollbar()
        .child(
            // 数值组：标签列宽固定，控件紧随其后，纵向对齐
            div()
                .flex_col()
                .gap(px(GAP_S))
                .child(number_row(t_timeout, t_sec, &timeout_input, &theme))
                .child(number_row(t_retries, t_times, &retry_input, &theme)),
        )
        .child(section_divider(&theme))
        .child(toggle_row("req-follow-redirects", t_follow, settings.follow_redirects, &theme, cx, |this, cx| {
            this.toggle_follow_redirects(cx);
        }))
        .child(toggle_row("req-verify-ssl", t_verify, settings.verify_ssl, &theme, cx, |this, cx| {
            this.toggle_verify_ssl(cx);
        }))
        .into_any_element()
}

/// 数值输入行：输入框 + 单位（单位用弱化色，避免和标签抢视线）
fn number_row(
    label: impl Into<SharedString>,
    unit: impl Into<SharedString>,
    state: &Entity<InputState>,
    theme: &Theme,
) -> AnyElement {
    let unit: SharedString = unit.into();
    form_row(
        label,
        theme,
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(GAP_S))
            .child(
                Input::new(state)
                    // 原来比其他输入框矮 2px（CONTROL_H - 2.0），统一成 CONTROL_H
                    .h(px(CONTROL_H))
                    .w(px(96.0))
                    .rounded(px(RADIUS_SM))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.control_bg())
                    .text_color(theme.foreground),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(unit),
            )
            .into_any_element(),
    )
}

/// 开关行：标签 + 统一开关组件
fn toggle_row(
    id: &'static str,
    label: impl Into<SharedString>,
    value: bool,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_toggle: impl Fn(&mut MainView, &mut Context<MainView>) + 'static,
) -> AnyElement {
    form_row(
        label,
        theme,
        toggle_switch(id, value, theme)
            .on_mouse_down(MouseButton::Left, cx.listener(
                move |this,
                      _: &MouseDownEvent,
                      _window: &mut Window,
                      cx: &mut Context<MainView>| {
                    on_toggle(this, cx);
                },
            ))
            .into_any_element(),
    )
}
