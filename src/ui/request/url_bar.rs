use crate::ui::components::{
    ghost_button, method_color, primary_button, themed_icon, IconTier, IconTone, CONTROL_H, GAP_S,
    ICON_TEXT_GAP, PANEL_PAD, RADIUS_SM,
};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::select::Select;
use gpui_component::{IconName, Sizable, StyledExt};

pub fn render_url_bar(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = this.cached_theme.clone();
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
        .gap(px(GAP_S))
        .px(px(PANEL_PAD))
        .py(px(GAP_S))
        .bg(theme.background)
        .border_b(px(1.0))
        .border_color(theme.border)
        .children([
            // HTTP 方法：定宽，避免下拉框宽度随方法名变化
            div()
                .h(px(CONTROL_H))
                .w(px(96.0))
                .flex_shrink_0()
                .flex()
                .child(
                    Select::new(&this.method_select)
                        .small()
                        .h(px(CONTROL_H))
                        .w_full()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(RADIUS_SM))
                        // 方法颜色由列表项(MethodItem)自带：Select 外层的 text_color 不作用于选中文字
                        .flex_none(),
                )
                .into_any_element(),
            // URL 输入
            div()
                .flex_1()
                .min_w(px(0.0))
                .h(px(CONTROL_H))
                .flex()
                .child(
                    Input::new(&this.url_input)
                        .h(px(CONTROL_H))
                        .w_full()
                        .bg(theme.control_bg())
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(RADIUS_SM))
                        .text_sm()
                        .text_color(theme.foreground),
                )
                .into_any_element(),
            // 次要操作：保存（描边按钮，放在发送左边）
            ghost_button("save-request", this.t("button.save"), &theme).on_click(cx.listener(
                |this: &mut MainView,
                 _: &gpui::ClickEvent,
                 window: &mut Window,
                 cx: &mut Context<MainView>| {
                    this.save_current_request(window, cx);
                },
            ))
            .into_any_element(),
            // 主要操作：发送（主色实心，固定在最右侧）
            primary_button(
                "send",
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(ICON_TEXT_GAP))
                    // 实心主色按钮内的图标 → Inherit（跟按钮的 accent_foreground）；
                    // 图标与文字之间一律 ICON_TEXT_GAP
                    .child(themed_icon(
                        if is_loading {
                            IconName::LoaderCircle
                        } else {
                            IconName::Play
                        },
                        IconTier::Dense,
                        IconTone::Inherit,
                        &theme,
                    ))
                    .child(if is_loading {
                        this.t("ui.sending")
                    } else {
                        this.t("ui.send")
                    }),
                &theme,
            )
            .min_w(px(96.0))
            .on_click(cx.listener(
                |this: &mut MainView,
                 _: &gpui::ClickEvent,
                 window: &mut Window,
                 cx: &mut Context<MainView>| {
                    let url = this.url_input.read(cx).value().to_string();
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
            ))
            .into_any_element(),
        ])
}
