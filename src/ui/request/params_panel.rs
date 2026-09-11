use crate::ui::components::{themed_icon, IconTier, IconTone, CONTROL_H, ICON_BTN, RADIUS_SM};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
// `when` 等流式构造方法来自 prelude（与 main_view.rs 保持一致）
use gpui::prelude::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::{IconName, Sizable, StyledExt};
use gpui_component::scroll::ScrollableElement;

pub fn render_params_panel(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = this.cached_theme.clone();
    let hover_bg = theme.hover_bg();
    let active_bg = theme.active_bg();
    // 颜色值先取出来：`theme` 现在是 Arc，move 闭包里没法像以前那样只捕获
    // `theme.error` 这一个 Copy 字段（Deref 之后取字段只能整个 Arc 移动进闭包），
    // 提前取出 Rgba 既避免移动 Arc，也不产生任何分配。
    let error_color = theme.error;

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
            // 表头与首行输入之间只保留外层 .gap_2() 的 8px：
            // 原来这里还叠了表头 .mb_1()、本容器 .py_1()/.mt_1()、每行 .mt_1()，合计约 24px，
            // 标题与输入框看起来「隔太开」。
            div()
                .flex_col()
                .gap_2()
                .children(this.params.iter().enumerate().map(|(idx, param)| {
                    // 首行不加顶部外边距：表头到首行只留外层 .gap_2() 的 8px；
                    // 其余行保留 4px，输入框之间维持原来的 12px 间距
                    div()
                        .when(idx > 0, |d| d.mt_1())
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
                                    .h(px(CONTROL_H))
                                    .bg(theme.control_bg())
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded(px(RADIUS_SM))
                                    .text_color(theme.foreground),
                            ),
                            div().flex_1().child(
                                Input::new(&param.value)
                                    .small()
                                    .h(px(CONTROL_H))
                                    .bg(theme.control_bg())
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded(px(RADIUS_SM))
                                    .text_color(theme.foreground),
                            ),
                            // 外层再包一层普通 div：children 数组要求元素类型一致
                            div().child(
                            div()
                                .id(format!("del-row-{}", idx))
                                // 行内图标按钮：尺寸/圆角取 ICON_BTN + RADIUS_SM，和 components::icon_button() 同一套
                                .w(px(ICON_BTN))
                                .h(px(ICON_BTN))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(RADIUS_SM))
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(move |s| s.bg(hover_bg).text_color(error_color))
                                .active(move |s| s.bg(active_bg).text_color(error_color))
                                .on_mouse_down(MouseButton::Left, cx.listener(
                                    move |this,
                                          _: &MouseDownEvent,
                                          _window: &mut Window,
                                          cx: &mut Context<MainView>| {
                                        this.remove_param(idx, _window, cx);
                                    },
                                ))
                                // 行内删除按钮：常态跟容器 muted_foreground，
                                // hover/按下跟容器的 error 色 → Inherit；
                                // 字形取 Delete：这一行的动作是"删除该行"，
                                // Close（叉）在本应用专表"关闭"，删除只能有一种字形
                                .child(themed_icon(
                                    IconName::Delete,
                                    IconTier::Dense,
                                    IconTone::Inherit,
                                    &theme,
                                )),
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
