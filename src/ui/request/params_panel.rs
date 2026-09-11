use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use gpui_component::scroll::ScrollableElement;

pub fn render_params_panel(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = this.cached_theme.clone();
    let hover_bg = theme.muted_background;

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .p_3().pb_32()
        .overflow_y_scrollbar()
        .children([
            div()
                .flex()
                .flex_row()
                .gap_2()
                .mb_1()
                .children([
                    // 表头左右占位分别取 24/22：和数据行的复选框(w24)、删除按钮(w22)等宽，
                    // 这样居中的列标题正好落在下方输入框的正中间
                    div().w(px(24.0)).child(""),
                    div()
                        .flex_1()
                        .text_xs()
                        .text_center()
                        .text_color(theme.muted_foreground)
                        .child(this.t("ui.key")),
                    div()
                        .flex_1()
                        .text_xs()
                        .text_center()
                        .text_color(theme.muted_foreground)
                        .child(this.t("ui.value")),
                    div().w(px(22.0)).child(""),
                ]),
            div()
                .flex_col()
                .gap_2()
                .py_1()
                .mt_1()
                .children(this.params.iter().enumerate().map(|(idx, param)| {
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
                                .cursor_pointer()
                                .text_color(if param.enabled {
                                    theme.success
                                } else {
                                    theme.muted_foreground
                                })
                                .child(if param.enabled { "✓" } else { "○" })
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                                    this.toggle_param(idx, window, cx);
                                })),
                            div().flex_1().child(
                                Input::new(&param.key)
                                    .small()
                                    .h(px(32.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            div().flex_1().child(
                                Input::new(&param.value)
                                    .small()
                                    .h(px(32.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                            ),
                            // 外层再包一层普通 div：children 数组要求元素类型一致
                            div().child(
                            div()
                                .id(format!("del-row-{}", idx))
                                .w(px(22.0))
                                .h(px(22.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_md()
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(move |s| s.bg(hover_bg).text_color(theme.error))
                                .on_mouse_down(MouseButton::Left, cx.listener(
                                    move |this,
                                          _: &MouseDownEvent,
                                          _window: &mut Window,
                                          cx: &mut Context<MainView>| {
                                        this.remove_param(idx, _window, cx);
                                    },
                                ))
                                .child(Icon::new(IconName::Close).xsmall()),
                            ),
                        ])
                })),
            div().mt_2().child(
                crate::ui::components::icon_text_button(
                    "add-param",
                    IconName::Plus,
                    this.t("ui.add_param"),
                    &theme,
                    theme.accent,
                )
                    .on_click(cx.listener(
                        |this: &mut MainView,
                         _: &ClickEvent,
                         window: &mut Window,
                         cx: &mut Context<MainView>| {
                            this.add_param(window, cx);
                        },
                    )),
            ),
        ])
}
