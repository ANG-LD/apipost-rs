//! 环境变量编辑对话框
//!
//! 支持创建/编辑环境、管理当前环境变量和全局变量

use crate::app::database::Environment;
use crate::app::AppState;
use crate::ui::components::{
    danger_button, ghost_button, icon_button, icon_text_button, primary_button, primary_button_sm,
    CONTROL_H,
    GAP_M, GAP_S, GAP_XS,
};
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{Disableable, Icon, IconName, Sizable, StyledExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 预分配的变量行槽位数
const MAX_VAR_SLOTS: usize = 15;

/// 对话框状态（存储在 MainView 中）
pub struct EnvDialogState {
    pub visible: bool,
    pub is_edit: bool,
    pub is_global_only: bool,
    pub env_id: Option<String>,
    pub name_input: Entity<InputState>,
    pub current_var_count: usize,
    pub current_vars: Vec<(Entity<InputState>, Entity<InputState>)>,
    pub global_var_count: usize,
    pub global_vars: Vec<(Entity<InputState>, Entity<InputState>)>,
    /// 保存后置为 true，MainView 检测后刷新环境列表
    pub needs_refresh: bool,
    /// 环境名为空时高亮输入框并给出提示
    pub name_error: bool,
}

impl EnvDialogState {
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).default_value(""));
        let mut current_vars = Vec::with_capacity(MAX_VAR_SLOTS);
        let mut global_vars = Vec::with_capacity(MAX_VAR_SLOTS);
        for _ in 0..MAX_VAR_SLOTS {
            current_vars.push((
                cx.new(|cx| InputState::new(window, cx).default_value("")),
                cx.new(|cx| InputState::new(window, cx).default_value("")),
            ));
            global_vars.push((
                cx.new(|cx| InputState::new(window, cx).default_value("")),
                cx.new(|cx| InputState::new(window, cx).default_value("")),
            ));
        }
        Self {
            visible: false,
            is_edit: false,
            is_global_only: false,
            env_id: None,
            name_input,
            current_var_count: 0,
            current_vars,
            global_var_count: 1,
            global_vars,
            needs_refresh: false,
            name_error: false,
        }
    }

    /// 填充对话框数据（环境变量模式）
    pub fn load_from(
        &mut self,
        env: Option<&Environment>,
        window: &mut Window,
        cx: &mut Context<crate::ui::MainView>,
    ) {
        self.env_id = env.map(|e| e.id.clone());
        self.is_edit = env.is_some();
        self.is_global_only = false;
        self.name_error = false;

        if let Some(env) = env {
            self.name_input.update(cx, |s, cx| {
                s.set_value(&env.name, window, cx);
            });
            if let Ok(vars) = serde_json::from_str::<HashMap<String, String>>(&env.variables) {
                // 至少留一行，空环境也能直接开始输入
                self.current_var_count = vars.len().min(MAX_VAR_SLOTS).max(1);
                for (i, (k, v)) in vars.iter().enumerate() {
                    if i >= MAX_VAR_SLOTS { break; }
                    let (ref ki, ref vi) = self.current_vars[i];
                    ki.update(cx, |s, cx| s.set_value(k, window, cx));
                    vi.update(cx, |s, cx| s.set_value(v, window, cx));
                }
            }
        } else {
            self.name_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.current_var_count = 1;
        }
        // 清除多余的当前环境变量槽位（避免上次编辑的残留数据）
        for i in self.current_var_count..MAX_VAR_SLOTS {
            let (ref ki, ref vi) = self.current_vars[i];
            ki.update(cx, |s, cx| s.set_value("", window, cx));
            vi.update(cx, |s, cx| s.set_value("", window, cx));
        }

        // 清除全局变量槽位
        self.global_var_count = 0;
        for i in 0..MAX_VAR_SLOTS {
            let (ref ki, ref vi) = self.global_vars[i];
            ki.update(cx, |s, cx| s.set_value("", window, cx));
            vi.update(cx, |s, cx| s.set_value("", window, cx));
        }

        self.visible = true;
    }

    /// 打开全局变量编辑模式
    pub fn open_global(
        &mut self,
        globals: &HashMap<String, String>,
        window: &mut Window,
        cx: &mut Context<crate::ui::MainView>,
    ) {
        self.is_global_only = true;
        self.is_edit = false;
        self.env_id = None;
        self.name_error = false;
        self.name_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.current_var_count = 0;
        for i in 0..MAX_VAR_SLOTS {
            let (ref ki, ref vi) = self.current_vars[i];
            ki.update(cx, |s, cx| s.set_value("", window, cx));
            vi.update(cx, |s, cx| s.set_value("", window, cx));
        }
        self.global_var_count = (globals.len().max(1)).min(MAX_VAR_SLOTS);
        for (i, (k, v)) in globals.iter().enumerate() {
            if i >= MAX_VAR_SLOTS { break; }
            let (ref ki, ref vi) = self.global_vars[i];
            ki.update(cx, |s, cx| s.set_value(k, window, cx));
            vi.update(cx, |s, cx| s.set_value(v, window, cx));
        }
        for i in self.global_var_count..MAX_VAR_SLOTS {
            let (ref ki, ref vi) = self.global_vars[i];
            ki.update(cx, |s, cx| s.set_value("", window, cx));
            vi.update(cx, |s, cx| s.set_value("", window, cx));
        }
        self.visible = true;
    }

    /// 收集当前环境变量为 HashMap
    pub fn collect_current_vars(&self, cx: &Context<crate::ui::MainView>) -> HashMap<String, String> {
        (0..self.current_var_count)
            .filter_map(|i| {
                let (ref k, ref v) = self.current_vars[i];
                let key = k.read(cx).value().to_string();
                let val = v.read(cx).value().to_string();
                if key.trim().is_empty() { None } else { Some((key, val)) }
            })
            .collect()
    }

    /// 收集全局变量为 HashMap
    pub fn collect_global_vars(&self, cx: &Context<crate::ui::MainView>) -> HashMap<String, String> {
        (0..self.global_var_count)
            .filter_map(|i| {
                let (ref k, ref v) = self.global_vars[i];
                let key = k.read(cx).value().to_string();
                let val = v.read(cx).value().to_string();
                if key.trim().is_empty() { None } else { Some((key, val)) }
            })
            .collect()
    }
}

/// 渲染环境编辑对话框（作为 MainView render 的一部分）
///
/// 当 dialog.visible 为 true 时渲染模态对话框覆盖层
pub fn render_env_dialog_overlay(
    state: &Arc<Mutex<EnvDialogState>>,
    app_state: &Arc<Mutex<AppState>>,
    active_env_id: &Option<String>,
    theme: &Theme,
    entity_id: gpui::EntityId,
    cx: &mut Context<crate::ui::MainView>,
) -> gpui::AnyElement {
    let st = state.lock().unwrap();
    if !st.visible {
        return div().into_any_element();
    }

    let t = |key: &str| -> String {
        app_state
            .lock()
            .map(|app| app.t(key))
            .unwrap_or_else(|_| key.to_string())
    };

    let is_global_only = st.is_global_only;
    let title = if st.is_global_only {
        // 原来这里写死了中文，英文界面下不会切换
        t("env.global_vars")
    } else if st.is_edit {
        t("env.edit")
    } else {
        t("env.create")
    };

    let name_input = st.name_input.clone();
    let current_var_count = st.current_var_count;
    let global_var_count = st.global_var_count;
    let current_slots: Vec<(Entity<InputState>, Entity<InputState>)> = st.current_vars.clone();
    let global_slots: Vec<(Entity<InputState>, Entity<InputState>)> = st.global_vars.clone();
    // 名称当前是否为空（用户输入后错误态自动消失，不必等再次点击保存）
    let name_empty = st.name_input.read(cx).value().trim().is_empty();
    let st_name_error = st.name_error && name_empty;
    let is_edit = st.is_edit;
    let editing_env_id = st.env_id.clone();
    drop(st);

    let state_c = state.clone();

    let save_state = state.clone();
    let save_app = app_state.clone();
    let save_active = active_env_id.clone();
    let save_entity_id = entity_id;

    let env_name_label = t("env.name_label");
    let current_vars_label = t("env.current_vars");
    let global_vars_label = t("env.global_vars");
    let add_var_label = t("env.add");
    let cancel_label = t("env.cancel");
    let save_env_label = t("env.save_env");
    let var_name_label = t("env.name");
    let var_value_label = t("env.value");
    let empty_hint = t("env.empty_hint");
    let name_required_label = t("env.name_required");
    let vars_count = |n: usize| t("env.vars_count").replace("{}", &n.to_string());
    let name_error = st_name_error;

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .bg(rgba(0x00000055))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .w(px(640.0))
                .bg(theme.background)
                .rounded_xl()
                .border_1()
                .border_color(theme.border)
                .shadow_2xl()
                .flex_col()
                .overflow_hidden()
                // 标题栏
                .child(
                    div()
                        .h(px(48.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px_5()
                        .bg(theme.muted_background)
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2p5()
                                .child(Icon::new(IconName::Globe).text_color(theme.accent))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight(600.0))
                                        .text_color(theme.foreground)
                                        .child(title.clone()),
                                ),
                        )
                        .child({
                            let s = state_c.clone();
                            let eid = entity_id;
                            icon_button(
                                "close-env-dialog",
                                IconName::Close,
                                theme,
                                theme.muted_foreground,
                                theme.foreground,
                            )
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                if let Ok(mut st) = s.lock() {
                                    st.visible = false;
                                }
                                cx.notify(eid);
                            })
                        }),
                )
                // 内容区：高度跟随变量行数，新建时不会留一大片空白
                .child(
                    div()
                        .h(px(if is_global_only {
                            (180.0 + 30.0 * global_var_count.max(1) as f32).min(420.0)
                        } else {
                            (180.0 + 30.0 * current_var_count.max(1) as f32).min(420.0)
                        }))
                        .px_5()
                        .py_4()
                        .flex_col()
                        .gap_5()
                        .overflow_y_scrollbar()
                        // 环境名称（仅非全局模式显示）
                        .when(!is_global_only, |d| {
                            d.child(
                                div().flex_col().gap(px(GAP_XS))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(GAP_XS))
                                            .child(Icon::new(IconName::Globe).xsmall().text_color(theme.accent))
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_weight(FontWeight(600.0))
                                                    .text_color(theme.muted_foreground)
                                                    .child(env_name_label.clone()),
                                            ),
                                    )
                                    .child(
                                        Input::new(&name_input)
                                            .h(px(CONTROL_H))
                                            .w_full()
                                            .rounded_md()
                                            .border_1()
                                            // 名称为空时红框提示，而不是点了保存什么反应都没有
                                            .border_color(if name_error { theme.error } else { theme.border })
                                            .bg(theme.input_background)
                                            .text_color(theme.foreground),
                                    )
                                    .when(name_error, |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(theme.error)
                                                .child(name_required_label.clone()),
                                        )
                                    }),
                            )
                        })
                        // 当前环境变量（仅非全局模式显示）
                        .when(!is_global_only, |d| {
                            d.child(render_var_section(
                                &state_c, &current_vars_label,
                                "add-cur", true, current_var_count,
                                &current_slots, theme, entity_id,
                                &add_var_label, &var_name_label, &var_value_label, &empty_hint,
                                &vars_count(current_var_count),
                            ))
                        })
                        // 全局变量（仅全局模式显示）
                        .when(is_global_only, |d| {
                            d.child(render_var_section(
                                &state_c, &global_vars_label,
                                "add-glob", false, global_var_count,
                                &global_slots, theme, entity_id,
                                &add_var_label, &var_name_label, &var_value_label, &empty_hint,
                                &vars_count(global_var_count),
                            ))
                        }),
                )
                // 底部按钮栏
                .child(
                    div()
                        .h(px(48.0))
                        .flex()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .gap(px(GAP_M))
                        .px_5()
                        .border_t_1()
                        .border_color(theme.border)
                        .bg(theme.muted_background)
                        // 编辑已有环境时提供删除入口（原来只藏在列表的「⋯」菜单里）
                        .child(if is_edit && !is_global_only {
                            let del_app = app_state.clone();
                            let del_state = state_c.clone();
                            let del_id = editing_env_id.clone();
                            let eid = entity_id;
                            danger_button(
                                "delete-env-btn",
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(GAP_XS))
                                    .child(Icon::new(IconName::Delete).xsmall())
                                    .child(t("env.delete")),
                                theme,
                            )
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                if let Some(id) = del_id.as_ref() {
                                    if let Ok(app) = del_app.lock() {
                                        if let Err(e) = app.db.delete_environment(id) {
                                            log::error!("删除环境失败: {}", e);
                                        }
                                    }
                                }
                                if let Ok(mut st) = del_state.lock() {
                                    st.visible = false;
                                    st.needs_refresh = true;
                                }
                                cx.notify(eid);
                            })
                            .into_any_element()
                        } else {
                            div().into_any_element()
                        })
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(GAP_M))
                                .child({
                            let s = state_c.clone();
                            let eid = entity_id;
                            ghost_button("cancel-env-btn", cancel_label.clone(), theme)
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    if let Ok(mut st) = s.lock() {
                                        st.visible = false;
                                    }
                                    cx.notify(eid);
                                })
                        })
                        .child(
                            primary_button(
                                "save-env-btn",
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(GAP_XS))
                                    .child(Icon::new(IconName::Check).xsmall())
                                    .child(save_env_label.clone()),
                                theme,
                            )
                            .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                    let mut st = save_state.lock().unwrap();
                                    let is_global_only = st.is_global_only;
                                    let env_name = st.name_input.read(cx).value().to_string();
                                    if !is_global_only && env_name.trim().is_empty() {
                                        // 提示“名称不能为空”，而不是静默失败
                                        st.name_error = true;
                                        drop(st);
                                        cx.notify(save_entity_id);
                                        return;
                                    }
                                    let current_map: HashMap<String, String> = (0..st.current_var_count)
                                        .filter_map(|i| {
                                            let (ref k, ref v) = st.current_vars[i];
                                            let key = k.read(cx).value().to_string();
                                            let val = v.read(cx).value().to_string();
                                            if key.trim().is_empty() { None } else { Some((key, val)) }
                                        })
                                        .collect();
                                    let global_map: HashMap<String, String> = (0..st.global_var_count)
                                        .filter_map(|i| {
                                            let (ref k, ref v) = st.global_vars[i];
                                            let key = k.read(cx).value().to_string();
                                            let val = v.read(cx).value().to_string();
                                            if key.trim().is_empty() { None } else { Some((key, val)) }
                                        })
                                        .collect();
                                    let env_id = st.env_id.clone();
                                    let is_active = env_id.as_ref().and_then(|id| {
                                        if save_active.as_ref() == Some(id) { Some(true) } else { None }
                                    });
                                    let now = chrono::Utc::now();
                                    drop(st);

                                    if let Ok(app) = save_app.lock() {
                                        if is_global_only {
                                            // 仅保存全局变量
                                            if let Err(e) = app.db.save_global_variables(&global_map) {
                                                log::error!("保存全局变量失败: {}", e);
                                            }
                                            app.env_manager.set_globals(global_map);
                                        } else {
                                            let env_id = env_id.unwrap_or_else(|| {
                                                uuid::Uuid::new_v4().to_string()
                                            });
                                            let created_at = if let Ok(envs) = app.db.get_environments() {
                                                envs.iter().find(|e| e.id == env_id)
                                                    .map(|e| e.created_at)
                                                    .unwrap_or(now)
                                            } else { now };

                                            let env = Environment {
                                                id: env_id.clone(),
                                                name: env_name,
                                                variables: serde_json::to_string(&current_map).unwrap_or_default(),
                                                is_active: is_active.unwrap_or(false),
                                                is_global: false,
                                                created_at,
                                                updated_at: now,
                                            };
                                            if let Err(e) = app.db.save_environment(&env) {
                                                log::error!("保存环境失败: {}", e);
                                            }
                                            if is_active.unwrap_or(false) {
                                                if let Err(e) = app.env_manager.load_from_json(&env.variables) {
                                                    log::warn!("重载环境变量失败: {}", e);
                                                }
                                            }
                                        }
                                    }

                                    if let Ok(mut st) = save_state.lock() {
                                        st.visible = false;
                                        st.needs_refresh = true;
                                    }
                                    log::info!("环境保存成功");
                                    cx.notify(save_entity_id);
                                }),
                        ),
                        ),
                ),
        )
        .into_any_element()
}

/// 删除第 i 行变量：后面的行整体上移，最后一行清空
///
/// 变量输入框是预先分配好的固定槽位（MAX_VAR_SLOTS），所以删除只能靠“搬移”，
/// 原来这段逻辑在渲染闭包里写了两遍（当前变量 + 全局变量），这里收拢成一处。
fn remove_var_row(
    st: &mut EnvDialogState,
    is_current: bool,
    i: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let (count, slots) = if is_current {
        (st.current_var_count, st.current_vars.clone())
    } else {
        (st.global_var_count, st.global_vars.clone())
    };
    if i >= count {
        return;
    }
    for j in i..count.saturating_sub(1) {
        let (ref nk, ref nv) = slots[j + 1];
        let (ref ck, ref cv) = slots[j];
        let (nk_v, nv_v) = (nk.read(cx).value().to_string(), nv.read(cx).value().to_string());
        ck.update(cx, |s, cx| s.set_value(&nk_v, window, cx));
        cv.update(cx, |s, cx| s.set_value(&nv_v, window, cx));
    }
    if let Some((lk, lv)) = slots.get(count - 1) {
        lk.update(cx, |s, cx| s.set_value("", window, cx));
        lv.update(cx, |s, cx| s.set_value("", window, cx));
    }
    if is_current {
        st.current_var_count -= 1;
    } else {
        st.global_var_count -= 1;
    }
}

/// 变量编辑区：标题（图标 + 名称 + 数量 + 添加按钮）+ 变量卡片
fn render_var_section(
    state: &Arc<Mutex<EnvDialogState>>,
    label: &str,
    btn_id: &'static str,
    is_current: bool,
    count: usize,
    slots: &[(Entity<InputState>, Entity<InputState>)],
    theme: &Theme,
    entity_id: gpui::EntityId,
    add_label: &str,
    key_label: &str,
    value_label: &str,
    empty_hint: &str,
    count_label: &str,
) -> gpui::Div {
    let s_add = state.clone();
    let s_add_empty = state.clone();
    let s_del = state.clone();
    let label = label.to_string();
    let add_label = add_label.to_string();
    let key_label = key_label.to_string();
    let value_label = value_label.to_string();
    let empty_hint = empty_hint.to_string();
    let count_label = count_label.to_string();
    let at_cap = count >= MAX_VAR_SLOTS;
    let input_h = CONTROL_H - 4.0;

    div()
        .flex_col()
        .gap(px(GAP_S))
        // 节标题行
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(GAP_S))
                .child(
                    Icon::new(if is_current { IconName::Globe } else { IconName::Star })
                        .small()
                        .text_color(theme.accent),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight(600.0))
                        .text_color(theme.foreground)
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(count_label),
                )
                .child(div().flex_1())
                // 槽位用满时不再显示添加按钮
                .when(!at_cap, |d| {
                    d.child(
                        primary_button_sm(
                            btn_id,
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(2.0))
                                .text_size(px(11.0))
                                .child(Icon::new(IconName::Plus).xsmall())
                                .child(add_label.clone()),
                            theme,
                        )
                            // 上下各留 8px，不和上方输入框、下方变量卡片贴在一起
                            .my(px(8.0))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                if let Ok(mut st) = s_add.lock() {
                                    if is_current {
                                        if st.current_var_count < MAX_VAR_SLOTS {
                                            st.current_var_count += 1;
                                        }
                                    } else if st.global_var_count < MAX_VAR_SLOTS {
                                        st.global_var_count += 1;
                                    }
                                }
                                cx.notify(entity_id);
                            }),
                    )
                }),
        )
        // 变量卡片
        .child(
            div()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .overflow_hidden()
                .flex_col()
                // 列头
                .when(count > 0, |d| {
                    d.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(GAP_S))
                            .px(px(GAP_S + 2.0))
                            .py(px(GAP_XS))
                            .bg(theme.muted_background)
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight(600.0))
                                    .text_color(theme.muted_foreground)
                                    .child(key_label.clone()),
                            )
                            .child(
                                div()
                                    .w(px(input_h))
                                    .flex_1()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight(600.0))
                                    .text_color(theme.muted_foreground)
                                    .child(value_label.clone()),
                            )
                            .child(div().w(px(22.0))),
                    )
                })
                .children((0..count).map(move |i| {
                    let (ref k, ref v) = slots[i];
                    let s = s_del.clone();
                    let theme = theme.clone();
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(GAP_S))
                        .px(px(GAP_S + 2.0))
                        .py(px(GAP_S))
                        .when(i > 0, |d| d.border_t_1().border_color(theme.border))
                        .child(
                            Input::new(k)
                                .h(px(input_h))
                                .flex_1()
                                .rounded_md()
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.input_background)
                                .text_color(theme.foreground),
                        )
                        .child(
                            Input::new(v)
                                .h(px(input_h))
                                .flex_1()
                                .rounded_md()
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.input_background)
                                .text_color(theme.foreground),
                        )
                        .child(
                            icon_button(
                                SharedString::from(format!(
                                    "del-var-{}-{}",
                                    if is_current { "c" } else { "g" },
                                    i
                                )),
                                IconName::Delete,
                                &theme,
                                theme.muted_foreground,
                                theme.error,
                            )
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                if let Ok(mut st) = s.lock() {
                                    remove_var_row(&mut st, is_current, i, window, cx);
                                }
                                cx.notify(entity_id);
                            }),
                        )
                }))
                // 空状态：整块可点，直接加一行变量
                .when(count == 0, |d| {
                    let s_empty = s_add_empty.clone();
                    d.child(
                        div()
                            .id(SharedString::from(format!("empty-{}-add", btn_id)))
                            .w_full()
                            .px(px(GAP_S + 2.0))
                            .py(px(GAP_S))
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_center()
                            .gap(px(GAP_XS))
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(11.0))
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .hover(move |st| st.bg(theme.muted_background))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                if let Ok(mut st) = s_empty.lock() {
                                    if is_current {
                                        if st.current_var_count < MAX_VAR_SLOTS {
                                            st.current_var_count += 1;
                                        }
                                    } else if st.global_var_count < MAX_VAR_SLOTS {
                                        st.global_var_count += 1;
                                    }
                                }
                                cx.notify(entity_id);
                            })
                            .child(Icon::new(IconName::Plus).xsmall())
                            .child(add_label.clone())
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.muted_foreground)
                                    .child(empty_hint.clone()),
                            ),
                    )
                }),
        )
}
