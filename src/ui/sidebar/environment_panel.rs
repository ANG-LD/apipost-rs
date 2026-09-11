use crate::app::database::Environment;
use crate::app::AppState;
use crate::ui::components::{
    icon_button, icon_text_button, primary_button, section_divider, GAP_S, GAP_XS,
};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 解析环境里存成 JSON 字符串的变量，按键名排序
fn parse_vars(json: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = serde_json::from_str::<HashMap<String, String>>(json)
        .unwrap_or_default()
        .into_iter()
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// 按键名排序后的全局变量
fn sorted_globals(map: HashMap<String, String>) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = map.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// 按显示宽度截断（中文≈2 宽，英文≈1 宽）
fn truncate_display(s: &str, max_width: usize) -> String {
    let mut w = 0usize;
    for (i, ch) in s.char_indices() {
        w += if ch.is_ascii() { 1 } else { 2 };
        if w > max_width {
            return format!("{}…", &s[..i]);
        }
    }
    s.to_string()
}

/// 侧栏「环境变量」页：环境列表 + 当前环境变量 + 全局变量入口
///
/// 每个环境行右侧都有**常驻的编辑按钮**，变量区标题右侧也有「编辑变量」入口。
/// 原来只有一个不显眼的「⋯」按钮、菜单里还只有「重命名/删除」，
/// 看起来就像根本没有编辑环境变量的入口。
/// 环境面板标题行高度（固定，便于算出下面滚动区的高度）
const ENV_TITLE_H: f32 = 36.0;

pub fn render_environment_panel(
    environments: &[Environment],
    active_env_id: Option<String>,
    app_state: &Arc<Mutex<AppState>>,
    theme: &Theme,
    // 侧边栏内容区的可用高度（由 main_view 按窗口高度算出）。
    // 这条高度链在 gpui 里是 auto：flex_1/100% 都拿不到确定高度，
    // 必须显式给高度，否则内容会把容器撑高、超出窗口被裁掉，且没有滚动范围。
    content_h: f32,
    cx: &mut Context<MainView>,
) -> Stateful<Div> {
    let t = |key: &str| -> String {
        app_state
            .lock()
            .map(|a| a.t(key))
            .unwrap_or_else(|_| key.to_string())
    };
    let globals = app_state
        .lock()
        .map(|a| sorted_globals(a.env_manager.get_all_globals()))
        .unwrap_or_default();

    let env_vars: Vec<Vec<(String, String)>> =
        environments.iter().map(|e| parse_vars(&e.variables)).collect();
    let active_index = environments
        .iter()
        .position(|e| Some(&e.id) == active_env_id.as_ref());
    let active_vars: Vec<(String, String)> = active_index
        .map(|i| env_vars[i].clone())
        .unwrap_or_default();
    let active_name: Option<String> = active_index.map(|i| environments[i].name.clone());

    // 激活态淡色底：给主题 accent 加透明度（不能用 muted_background —— 深色主题里它与卡片同色）
    let mut active_bg = theme.accent;
    active_bg.a = 0.18;
    let accent = theme.accent;
    let border = theme.border;
    let card_bg = theme.muted_background;
    let hover_bg = theme.muted_background;
    let fg = theme.foreground;
    let muted_fg = theme.muted_foreground;

    let vars_count = |n: usize| t("env.vars_count").replace("{}", &n.to_string());

    // ---------- 标题行 ----------
    let title_row = div()
        .h(px(ENV_TITLE_H))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP_XS))
        .px(px(GAP_S))
        .py(px(GAP_S))
        .child(Icon::new(IconName::Globe).small().text_color(accent))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(12.0))
                .font_weight(FontWeight(600.0))
                .text_color(fg)
                .child(t("sidebar.env")),
        )
        .child(
            icon_text_button(
                "sidebar-add-env",
                IconName::Plus,
                t("env.create"),
                theme,
                accent,
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                    this.open_environment_dialog(None, window, cx);
                }),
            ),
        );

    // ---------- 环境列表卡片 ----------
    let envs_card = div()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(border)
        .overflow_hidden()
        .children(environments.iter().enumerate().map(|(i, env)| {
            let click_id = env.id.clone();
            let edit_id = env.id.clone();
            let is_active = Some(&env.id) == active_env_id.as_ref();
            let name = truncate_display(&env.name, 14);
            let count = vars_count(env_vars[i].len());

            div()
                .id(SharedString::from(format!("env-row-{}", env.id)))
                .relative()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(GAP_S))
                .pl(px(GAP_S + 2.0))
                .pr(px(GAP_XS))
                .py(px(7.0))
                .when(i > 0, |d| d.border_t_1().border_color(border))
                .when(is_active, |d| d.bg(active_bg))
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                        if this.active_env_id().as_deref() != Some(click_id.as_str()) {
                            this.activate_environment(&click_id, window, cx);
                        }
                    }),
                )
                // 激活指示：左侧主色竖条
                .when(is_active, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left(px(0.0))
                            .top(px(5.0))
                            .bottom(px(5.0))
                            .w(px(2.0))
                            .rounded_full()
                            .bg(accent),
                    )
                })
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .flex_shrink_0()
                        .rounded_full()
                        .when(is_active, |d| d.bg(accent))
                        .when(!is_active, |d| d.border_1().border_color(muted_fg)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_size(px(12.0))
                        .font_weight(if is_active { FontWeight(600.0) } else { FontWeight(400.0) })
                        .text_color(if is_active { fg } else { muted_fg })
                        .child(name),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(10.0))
                        .text_color(muted_fg)
                        .child(count),
                )
                // 常驻编辑按钮：点开就是该环境的变量编辑弹窗
                .child(
                    icon_button(
                        SharedString::from(format!("env-edit-{}", env.id)),
                        IconName::Settings2,
                        theme,
                        muted_fg,
                        accent,
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                            // 不要同时触发整行的“切换环境”
                            cx.stop_propagation();
                            this.open_environment_dialog(Some(edit_id.clone()), window, cx);
                        }),
                    ),
                )
        }));

    // ---------- 当前环境变量明细 ----------
    let vars_section = {
        let section_title = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(GAP_XS))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight(600.0))
                    .text_color(muted_fg)
                    .child(match &active_name {
                        Some(n) => format!("{} · {}", t("env.vars"), truncate_display(n, 10)),
                        None => t("env.vars"),
                    }),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(muted_fg)
                    .child(vars_count(active_vars.len())),
            )
            .child(div().flex_1())
            // 变量区的编辑入口（没有激活环境时不显示）
            .when(active_index.is_some(), |d| {
                let edit_id = active_env_id.clone().unwrap_or_default();
                d.child(
                    icon_text_button(
                        "sidebar-edit-vars",
                        IconName::Settings2,
                        t("env.vars_edit"),
                        theme,
                        accent,
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                            this.open_environment_dialog(Some(edit_id.clone()), window, cx);
                        }),
                    ),
                )
            });

        let body = if active_vars.is_empty() {
            div()
                .w_full()
                .px(px(GAP_S))
                .py(px(GAP_S))
                .rounded_md()
                .border_1()
                .border_color(border)
                .text_size(px(11.0))
                .text_color(muted_fg)
                .child(t("env.no_vars"))
                .into_any_element()
        } else {
            let edit_id = active_env_id.clone().unwrap_or_default();
            div()
                .id("sidebar-env-var-list")
                .flex_col()
                .rounded_md()
                .border_1()
                .border_color(border)
                .overflow_hidden()
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                        this.open_environment_dialog(Some(edit_id.clone()), window, cx);
                    }),
                )
                .children(active_vars.iter().enumerate().map(|(i, (k, v))| {
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(GAP_XS))
                        .px(px(GAP_S))
                        .py(px(4.0))
                        .when(i > 0, |d| d.border_t_1().border_color(border))
                        .when(i % 2 == 1, |d| d.bg(card_bg))
                        .child(
                            div()
                                .w(px(78.0))
                                .flex_shrink_0()
                                .overflow_hidden()
                                .text_size(px(11.0))
                                .text_color(fg)
                                .child(truncate_display(k, 16)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_size(px(11.0))
                                .text_color(muted_fg)
                                .child(truncate_display(v, 26)),
                        )
                }))
                .into_any_element()
        };

        div()
            .flex_col()
            .gap(px(GAP_XS))
            .child(section_title)
            .child(body)
    };

    // ---------- 全局变量入口 ----------
    let globals_row = div()
        .id("sidebar-global-vars")
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GAP_S))
        .px(px(GAP_S))
        .py(px(7.0))
        .rounded_md()
        .border_1()
        .border_color(border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.open_global_dialog(window, cx);
            }),
        )
        .child(Icon::new(IconName::Star).small().text_color(accent))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .text_size(px(12.0))
                .text_color(fg)
                .child(t("sidebar.global_vars")),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(10.0))
                .text_color(muted_fg)
                .child(vars_count(globals.len())),
        );

    // ---------- 组装 ----------
    div()
        .id("sidebar-environments")
        .relative()
        .flex_col()
        .h(px(content_h))
        // 解除 flex 子项自动最小高度(=内容高度)，否则显式高度会被内容顶开
        .min_h(px(0.0))
        .flex_shrink_0()
        .child(title_row)
        .child(
            div()
                .id("sidebar-env-scroll")
                .h(px((content_h - ENV_TITLE_H).max(80.0)))
                .min_h(px(0.0))
                .flex_shrink_0()
                .flex_col()
                // 用原生滚动：Scrollable 会给内容套 .size_auto().flex_1()，
                // flex 子项默认 flex_shrink=1 会被压成容器高度，可滚动范围就成了 0
                .overflow_y_scroll()
                .overflow_x_hidden()
                .child(
                    div()
                        // 内容层：保持自然高度，绝不能被压缩
                        .w_full()
                        .flex_none()
                        .flex_col()
                        .gap(px(GAP_S))
                        .px(px(GAP_S))
                        .pb(px(GAP_S))
                        .when(environments.is_empty(), |d| {
                            d.child(
                                div()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(GAP_S))
                                    .p(px(GAP_S + 4.0))
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(muted_fg)
                                            .child(t("env.empty_list")),
                                    )
                                    .child(
                                        primary_button(
                                            "sidebar-add-env-empty",
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(GAP_XS))
                                                .child(Icon::new(IconName::Plus).xsmall())
                                                .child(t("env.create")),
                                            theme,
                                        )
                                        .on_mouse_down(MouseButton::Left, cx.listener(
                                            |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                                                this.open_environment_dialog(None, window, cx);
                                            },
                                        )),
                                    ),
                            )
                        })
                        .when(!environments.is_empty(), |d| {
                            d.child(envs_card)
                                .child(vars_section)
                                .child(section_divider(theme))
                                .child(globals_row)
                        }),
                ),
        )
}
