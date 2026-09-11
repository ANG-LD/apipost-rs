use crate::app::database::Environment;
use crate::app::AppState;
use crate::ui::components::{
    icon_button, icon_text_button, primary_button, section_divider, themed_icon, IconTier, IconTone,
    GAP_S, GAP_XS, ICON_TEXT_GAP, RADIUS_MD, RADIUS_SM,
};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{IconName, StyledExt};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 解析环境里存成 JSON 字符串的变量，按键名排序
pub fn parse_vars(json: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = serde_json::from_str::<HashMap<String, String>>(json)
        .unwrap_or_default()
        .into_iter()
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// 「值是字符串」这一个约束的检查器：只判断类型，不物化字符串。
///
/// 用它是为了让快路径的接受范围与原 `HashMap<String, String>` 完全一致：
/// 值不是字符串（数字 / 对象 / 数组 / bool / null）时同样报错。
struct StrValue;

impl<'de> serde::de::Visitor<'de> for StrValue {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a string")
    }
    fn visit_str<E: serde::de::Error>(self, _v: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, _v: &'de str) -> Result<(), E> {
        Ok(())
    }
    fn visit_string<E: serde::de::Error>(self, _v: String) -> Result<(), E> {
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for StrValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(StrValue).map(|()| StrValue)
    }
}

/// 环境里有多少个变量（只数个数，不物化键值字符串）。
///
/// 侧栏每个环境行右侧只显示一个数字，改造前却是 `parse_vars(&e.variables).len()` ——
/// 每帧把每个环境的 JSON 反序列化成 `Vec<(String, String)>`（每个变量 2 次堆分配）
/// 再排一次序，只为取一个 `len()`。
///
/// 分两条路，保证语义与 `parse_vars(...).len()` **逐字一致**：
/// * 快路径：键按 `&str` 借用原文、值只检查「是不是字符串」——常见环境（键值都是普通
///   字符串、没有转义序列）零字符串分配，只花一张 HashMap 表；重复键的语义
///   （只算一次）由 HashMap 保证，与原来相同；
/// * 慢路径：一旦出现转义序列（`&str` 借用不到）、值不是字符串、或根本不是 JSON 对象，
///   就原样退回 `parse_vars` —— 那条路就是改造前的实现，结果必然一致。
pub(crate) fn variable_count(json: &str) -> usize {
    if let Ok(vars) = serde_json::from_str::<HashMap<&str, StrValue>>(json) {
        return vars.len();
    }
    parse_vars(json).len()
}

/// 按显示宽度截断（中文≈2 宽，英文≈1 宽）
///
/// 返回 `Cow`：没超宽时零分配借用原串。老写法返回 `String`，于是
/// 「本来就没超宽」的名字每帧每个都要白白分配一次；而 `String` 交给 `.child()`
/// 还会再转一次 `Arc<str>`（第二次分配），`SharedString::from(Cow::Borrowed(短串))`
/// 则是内联存储、一次分配都不要。
pub(crate) fn truncate_display(s: &str, max_width: usize) -> Cow<'_, str> {
    let mut w = 0usize;
    for (i, ch) in s.char_indices() {
        w += if ch.is_ascii() { 1 } else { 2 };
        if w > max_width {
            return Cow::Owned(format!("{}…", &s[..i]));
        }
    }
    Cow::Borrowed(s)
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
    // 翻译字典（`Arc<HashMap<String, Arc<str>>>`）：命中时取文案只是引用计数 +1。
    // 从 MainView 里传进来，替代原来「每次取文案都去锁 app_state 再 to_string()」——
    // 这一个面板每帧要取 ~9 条文案，原来就是 9 次加锁 + 9 次堆分配。
    translations: &Arc<crate::i18n::Translations>,
    theme: &Theme,
    // 侧边栏内容区的可用高度（由 main_view 按窗口高度算出）。
    // 这条高度链在 gpui 里是 auto：flex_1/100% 都拿不到确定高度，
    // 必须显式给高度，否则内容会把容器撑高、超出窗口被裁掉，且没有滚动范围。
    content_h: f32,
    cx: &mut Context<MainView>,
) -> Stateful<Div> {
    // 与 `MainView::t` 同一套零分配查表（同一个字典，结果逐字一致）
    let t = |key: &str| -> SharedString {
        match translations.get(key) {
            Some(value) => SharedString::from(Arc::clone(value)),
            // 只有翻译缺失才会走到这里，才会真的分配
            None => SharedString::from(key),
        }
    };
    // 全局变量区只显示一个「条数」：原来每帧 `get_all_globals()`（克隆整份 HashMap，
    // 每个键值各一次分配）+ 转 Vec + 排序，只为了最后取 `.len()`。计数语义完全相同。
    let globals_len = app_state
        .lock()
        .map(|a| a.env_manager.globals_count())
        .unwrap_or_default();

    let active_index = environments
        .iter()
        .position(|e| Some(&e.id) == active_env_id.as_ref());
    // 只有「当前环境」的键值需要真的解析出来显示。其它环境行只要一个数字，
    // 走 `variable_count`（只数个数）——原来每帧把**每个**环境的 JSON 都反序列化成
    // `Vec<(String, String)>` 再排序，非当前环境的那份结果只被 `.len()` 用过一次。
    let active_vars: Vec<(String, String)> = active_index
        .map(|i| parse_vars(&environments[i].variables))
        .unwrap_or_default();
    let active_name: Option<&str> = active_index.map(|i| environments[i].name.as_str());
    let var_counts: Vec<usize> = environments
        .iter()
        .map(|e| variable_count(&e.variables))
        .collect();

    // 激活态淡色底：给主题 accent 加透明度（不能用 muted_background —— 深色主题里它与卡片同色）
    let mut active_bg = theme.accent;
    active_bg.a = 0.18;
    let accent = theme.accent;
    let border = theme.border;
    let card_bg = theme.muted_background;
    let hover_bg = theme.muted_background;
    let fg = theme.foreground;
    let muted_fg = theme.muted_foreground;

    // 文案里有一个 "{}" 占位符：拼一次 String 交给 SharedString。
    // （翻译串本身来自 Arc，不再每帧分配）
    let vars_count = |n: usize| -> SharedString {
        SharedString::from(t("env.vars_count").replace("{}", &n.to_string()))
    };

    // ---------- 标题行 ----------
    let title_row = div()
        .h(px(ENV_TITLE_H))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(ICON_TEXT_GAP))
        .px(px(GAP_S))
        .py(px(GAP_S))
        // 侧栏小标题（相邻文字 12px）→ 密集档；"环境变量"是这一栏的主标识 → Accent
        .child(themed_icon(
            IconName::Globe,
            IconTier::Dense,
            IconTone::Accent,
            theme,
        ))
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
        // 卡片统一卡片档圆角（值不变，改为引用令牌）
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(border)
        .overflow_hidden()
        .children(environments.iter().enumerate().map(|(i, env)| {
            let click_id = env.id.clone();
            let edit_id = env.id.clone();
            let is_active = Some(&env.id) == active_env_id.as_ref();
            // SharedString::from(Cow) 对「没被截断」的名字是内联存储，零分配
            let name = SharedString::from(truncate_display(&env.name, 14));
            let count = vars_count(var_counts[i]);

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
                    .child(match active_name {
                        Some(n) => SharedString::from(format!(
                            "{} · {}",
                            t("env.vars"),
                            truncate_display(n, 10)
                        )),
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
                .rounded(px(RADIUS_SM))
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
                .rounded(px(RADIUS_SM))
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
                                .child(SharedString::from(truncate_display(k, 16))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_size(px(11.0))
                                .text_color(muted_fg)
                                .child(SharedString::from(truncate_display(v, 26))),
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
        .gap(px(ICON_TEXT_GAP))
        .px(px(GAP_S))
        .py(px(7.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(border)
        .cursor_pointer()
        // hover 只换底色：图标与文字都保持常态色，两者始终同色
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.open_global_dialog(window, cx);
            }),
        )
        // 与"当前环境"（Globe）区分开：这里表达的是"全局变量"，也是强调项 → Accent，
        // 与相邻 12px 文字同档。字形取 Asterisk（通配符 *）：本图标集没有变量/花括号字形，
        // "*"=不限定作用域（对所有请求生效）是最近的一档；Star 是"收藏"语义，不能用
        .child(themed_icon(
            IconName::Asterisk,
            IconTier::Dense,
            IconTone::Accent,
            theme,
        ))
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
                .child(vars_count(globals_len)),
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
                                    .rounded(px(RADIUS_MD))
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
                                                .gap(px(ICON_TEXT_GAP))
                                                // 实心主色按钮内的图标 → Inherit（跟按钮的 accent_foreground）
                                                .child(themed_icon(
                                                    IconName::Plus,
                                                    IconTier::Dense,
                                                    IconTone::Inherit,
                                                    theme,
                                                ))
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
