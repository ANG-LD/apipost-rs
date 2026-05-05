//! 环境变量编辑对话框
//!
//! 支持创建/编辑环境、管理当前环境变量和全局变量

use crate::app::database::Environment;
use crate::app::AppState;
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
    pub env_id: Option<String>,
    pub name_input: Entity<InputState>,
    pub current_var_count: usize,
    pub current_vars: Vec<(Entity<InputState>, Entity<InputState>)>,
    pub global_var_count: usize,
    pub global_vars: Vec<(Entity<InputState>, Entity<InputState>)>,
    /// 保存后置为 true，MainView 检测后刷新环境列表
    pub needs_refresh: bool,
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
            env_id: None,
            name_input,
            current_var_count: 0,
            current_vars,
            global_var_count: 0,
            global_vars,
            needs_refresh: false,
        }
    }

    /// 填充对话框数据
    pub fn load_from(
        &mut self,
        env: Option<&Environment>,
        globals: &HashMap<String, String>,
        window: &mut Window,
        cx: &mut Context<crate::ui::MainView>,
    ) {
        self.env_id = env.map(|e| e.id.clone());
        self.is_edit = env.is_some();

        if let Some(env) = env {
            self.name_input.update(cx, |s, cx| {
                s.set_value(&env.name, window, cx);
            });
            if let Ok(vars) = serde_json::from_str::<HashMap<String, String>>(&env.variables) {
                self.current_var_count = vars.len().min(MAX_VAR_SLOTS);
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
            self.current_var_count = 0;
        }
        // 清除多余的当前环境变量槽位（避免上次编辑的残留数据）
        for i in self.current_var_count..MAX_VAR_SLOTS {
            let (ref ki, ref vi) = self.current_vars[i];
            ki.update(cx, |s, cx| s.set_value("", window, cx));
            vi.update(cx, |s, cx| s.set_value("", window, cx));
        }

        self.global_var_count = globals.len().min(MAX_VAR_SLOTS);
        for (i, (k, v)) in globals.iter().enumerate() {
            if i >= MAX_VAR_SLOTS { break; }
            let (ref ki, ref vi) = self.global_vars[i];
            ki.update(cx, |s, cx| s.set_value(k, window, cx));
            vi.update(cx, |s, cx| s.set_value(v, window, cx));
        }
        // 清除多余的全局变量槽位
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

    let title = if st.is_edit {
        t("env.edit")
    } else {
        t("env.create")
    };

    let name_input = st.name_input.clone();
    let current_var_count = st.current_var_count;
    let global_var_count = st.global_var_count;
    let current_slots: Vec<(Entity<InputState>, Entity<InputState>)> = st.current_vars.clone();
    let global_slots: Vec<(Entity<InputState>, Entity<InputState>)> = st.global_vars.clone();
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

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .bg(rgba(0x00000044))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_, _, _| {})
        .child(
            div()
                .w(px(680.0))
                .bg(theme.background)
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .shadow_lg()
                .flex_col()
                .overflow_hidden()
                // 标题栏
                .child(
                    div()
                        .h(px(52.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px_6()
                        .bg(theme.muted_background)
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(36.0))
                                        .h(px(36.0))
                                        .rounded_md()
                                        .bg(theme.muted_background)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(Icon::new(IconName::Globe).text_color(theme.accent)),
                                )
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
                            Button::new("close-env-dialog")
                                .icon(IconName::Close)
                                .small()
                                .text_color(theme.muted_foreground)
                                .on_click(move |_, _, cx| {
                                    if let Ok(mut st) = s.lock() {
                                        st.visible = false;
                                    }
                                    cx.notify(eid);
                                })
                        }),
                )
                // 内容区（固定高度 + 超出滚动）
                .child(
                    div()
                        .h(px(380.0))
                        .px_6()
                        .py_5()
                        .flex_col()
                        .gap_6()
                        .overflow_y_scrollbar()
                        // 环境名称
                        .child(
                            div().flex_col().gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(Icon::new(IconName::File).small().text_color(theme.accent))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight(500.0))
                                                .text_color(theme.foreground)
                                                .child(env_name_label.clone()),
                                        ),
                                )
                                .child(
                                    Input::new(&name_input)
                                        .h(px(38.0))
                                        .w_full()
                                        .bg(theme.background)
                                        .text_color(theme.foreground),
                                ),
                        )
                        // 当前环境变量
                        .child(render_var_section(
                            &state_c, &current_vars_label,
                            "add-cur", true, current_var_count,
                            &current_slots, theme, entity_id, IconName::Globe,
                            &add_var_label, &var_name_label, &var_value_label, &empty_hint,
                        ))
                        // 全局变量
                        .child(render_var_section(
                            &state_c, &global_vars_label,
                            "add-glob", false, global_var_count,
                            &global_slots, theme, entity_id, IconName::Star,
                            &add_var_label, &var_name_label, &var_value_label, &empty_hint,
                        )),
                )
                // 底部按钮栏
                .child(
                    div()
                        .h(px(52.0))
                        .flex()
                        .flex_row()
                        .justify_end()
                        .items_center()
                        .gap_3()
                        .px_6()
                        .border_t_1()
                        .border_color(theme.border)
                        .bg(theme.muted_background)
                        .child({
                            let s = state_c.clone();
                            let eid = entity_id;
                            Button::new("cancel-env-btn")
                                .label(cancel_label.clone())
                                .on_click(move |_, _, cx| {
                                    if let Ok(mut st) = s.lock() {
                                        st.visible = false;
                                    }
                                    cx.notify(eid);
                                })
                        })
                        .child(
                            Button::new("save-env-btn")
                                .icon(IconName::Check)
                                .label(save_env_label.clone())
                                .bg(theme.accent)
                                .text_color(rgb(0xffffff))
                                .on_click(move |_, _window, cx| {
                                    let st = save_state.lock().unwrap();
                                    let env_name = st.name_input.read(cx).value().to_string();
                                    if env_name.trim().is_empty() {
                                        drop(st);
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
                                    let env_id = st.env_id.clone().unwrap_or_else(|| {
                                        uuid::Uuid::new_v4().to_string()
                                    });
                                    let is_active = save_active.as_ref().map_or(false, |id| *id == env_id);
                                    let now = chrono::Utc::now();
                                    drop(st);

                                    if let Ok(app) = save_app.lock() {
                                        let created_at = if let Ok(envs) = app.db.get_environments() {
                                            envs.iter().find(|e| e.id == env_id)
                                                .map(|e| e.created_at)
                                                .unwrap_or(now)
                                        } else { now };

                                        let env = Environment {
                                            id: env_id.clone(),
                                            name: env_name,
                                            variables: serde_json::to_string(&current_map).unwrap_or_default(),
                                            is_active,
                                            created_at,
                                            updated_at: now,
                                        };
                                        if let Err(e) = app.db.save_environment(&env) {
                                            log::error!("保存环境失败: {}", e);
                                        }
                                        if let Err(e) = app.db.save_global_variables(&global_map) {
                                            log::error!("保存全局变量失败: {}", e);
                                        }
                                        app.env_manager.set_globals(global_map);
                                        if is_active {
                                            if let Err(e) = app.env_manager.load_from_json(&env.variables) {
                                                log::warn!("重载环境变量失败: {}", e);
                                            }
                                        }
                                    }

                                    if let Ok(mut st) = save_state.lock() {
                                        st.visible = false;
                                        st.needs_refresh = true;
                                    }
                                    log::info!("环境保存成功: {}", env_id);
                                    cx.notify(save_entity_id);
                                }),
                        ),
                ),
        )
        .into_any_element()
}

fn render_var_section(
    state: &Arc<Mutex<EnvDialogState>>,
    label: &str,
    btn_id: &'static str,
    is_current: bool,
    count: usize,
    slots: &[(Entity<InputState>, Entity<InputState>)],
    theme: &Theme,
    entity_id: gpui::EntityId,
    section_icon: IconName,
    add_label: &str,
    key_label: &str,
    value_label: &str,
    empty_hint: &str,
) -> gpui::Div {
    let s_add = state.clone();
    let s_del = state.clone();
    let label = label.to_string();
    let add_label = add_label.to_string();
    let key_label = key_label.to_string();
    let value_label = value_label.to_string();
    let empty_hint = empty_hint.to_string();

    div()
        .flex_col()
        .gap_3()
        // 节标题行
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(section_icon).small().text_color(theme.accent))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight(500.0))
                                .text_color(theme.foreground)
                                .child(label.clone()),
                        ),
                )
                .child({
                    Button::new(btn_id)
                        .icon(IconName::Plus)
                        .label(add_label.clone())
                        .small()
                        .text_color(theme.accent)
                        .on_click(move |_, _, cx| {
                            if let Ok(mut st) = s_add.lock() {
                                if is_current {
                                    if st.current_var_count < MAX_VAR_SLOTS {
                                        st.current_var_count += 1;
                                    }
                                } else {
                                    if st.global_var_count < MAX_VAR_SLOTS {
                                        st.global_var_count += 1;
                                    }
                                }
                            }
                            cx.notify(entity_id);
                        })
                }),
        )
        // 变量卡片容器
        .child({
            if count == 0 {
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .p_5()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(empty_hint.clone())
            } else {
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .overflow_hidden()
                    .flex_col()
                    // 列头
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .px_4()
                            .py_2()
                            .bg(theme.muted_background)
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .font_weight(FontWeight(500.0))
                                    .text_color(theme.muted_foreground)
                                    .child(key_label.clone()),
                            )
                            .child(
                                div()
                                    .w(px(4.0)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .font_weight(FontWeight(500.0))
                                    .text_color(theme.muted_foreground)
                                    .child(value_label.clone()),
                            )
                            .child(
                                div()
                                    .w(px(36.0)),
                            ),
                    )
                    .children(
                        (0..count).map(move |i| {
                            let (ref k, ref v) = slots[i];
                            let s = s_del.clone();
                            let theme = theme.clone();
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .items_center()
                                .px_4()
                                .py_3()
                                .when(i > 0, |d| d.border_t_1().border_color(theme.border))
                                .bg(theme.background)
                                .child(
                                    Input::new(k)
                                        .h(px(36.0))
                                        .flex_1()
                                        .bg(theme.background)
                                        .text_color(theme.foreground),
                                )
                                .child(
                                    Input::new(v)
                                        .h(px(36.0))
                                        .flex_1()
                                        .bg(theme.background)
                                        .text_color(theme.foreground),
                                )
                                .child({
                                    Button::new(format!("del-var-{}-{}", if is_current { "c" } else { "g" }, i))
                                        .icon(IconName::Delete)
                                        .small()
                                        .text_color(theme.muted_foreground)
                                        .on_click(move |_, window, cx| {
                                            if let Ok(mut st) = s.lock() {
                                                if is_current && i < st.current_var_count {
                                                    // 将后续变量前移，并清空最后一个位置
                                                    for j in i..st.current_var_count.saturating_sub(1) {
                                                        let (ref next_k, ref next_v) = st.current_vars[j + 1].clone();
                                                        let (ref cur_k, ref cur_v) = st.current_vars[j].clone();
                                                        let nk = next_k.read(cx).value().to_string();
                                                        let nv = next_v.read(cx).value().to_string();
                                                        cur_k.update(cx, |s, cx| s.set_value(&nk, window, cx));
                                                        cur_v.update(cx, |s, cx| s.set_value(&nv, window, cx));
                                                    }
                                                    let last = st.current_var_count - 1;
                                                    let (ref lk, ref lv) = st.current_vars[last].clone();
                                                    lk.update(cx, |s, cx| s.set_value("", window, cx));
                                                    lv.update(cx, |s, cx| s.set_value("", window, cx));
                                                    st.current_var_count -= 1;
                                                } else if i < st.global_var_count {
                                                    for j in i..st.global_var_count.saturating_sub(1) {
                                                        let (ref next_k, ref next_v) = st.global_vars[j + 1].clone();
                                                        let (ref cur_k, ref cur_v) = st.global_vars[j].clone();
                                                        let nk = next_k.read(cx).value().to_string();
                                                        let nv = next_v.read(cx).value().to_string();
                                                        cur_k.update(cx, |s, cx| s.set_value(&nk, window, cx));
                                                        cur_v.update(cx, |s, cx| s.set_value(&nv, window, cx));
                                                    }
                                                    let last = st.global_var_count - 1;
                                                    let (ref lk, ref lv) = st.global_vars[last].clone();
                                                    lk.update(cx, |s, cx| s.set_value("", window, cx));
                                                    lv.update(cx, |s, cx| s.set_value("", window, cx));
                                                    st.global_var_count -= 1;
                                                }
                                            }
                                            cx.notify(entity_id);
                                        })
                                })
                        })
                    )
            }
        })
}
