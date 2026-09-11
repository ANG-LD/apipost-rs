/// 文件夹创建/重命名对话框
use crate::app::AppState;
use crate::ui::components::{
    button_size_for_icon, themed_icon, IconTier, IconTone, CONTROL_H, RADIUS_LG, RADIUS_SM,
};
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, IconName, Sizable, StyledExt};
use std::sync::{Arc, Mutex};

/// 文件夹/请求重命名对话框状态
pub struct FolderDialogState {
    pub visible: bool,
    pub is_edit: bool,
    pub is_request: bool,
    pub folder_id: Option<String>,
    pub request_id: Option<String>,
    pub parent_id: Option<String>,
    pub name_input: Entity<InputState>,
    pub needs_refresh: bool,
}

impl FolderDialogState {
    pub fn new(window: &mut Window, cx: &mut Context<crate::ui::MainView>) -> Self {
        Self {
            visible: false,
            is_edit: false,
            is_request: false,
            folder_id: None,
            request_id: None,
            parent_id: None,
            name_input: cx.new(|cx| InputState::new(window, cx).default_value("")),
            needs_refresh: false,
        }
    }

    /// 打开对话框用于创建文件夹
    pub fn open_for_create(&mut self, parent_id: Option<String>, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        log::info!("FolderDialogState::open_for_create, parent_id: {:?}", parent_id);
        self.visible = true;
        self.is_edit = false;
        self.is_request = false;
        self.folder_id = None;
        self.request_id = None;
        self.parent_id = parent_id;
        self.name_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }

    /// 打开对话框用于重命名文件夹
    pub fn open_for_edit(&mut self, folder_id: String, current_name: String, parent_id: Option<String>, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        log::info!("FolderDialogState::open_for_edit, folder_id={}, name={}", folder_id, current_name);
        self.visible = true;
        self.is_edit = true;
        self.is_request = false;
        self.folder_id = Some(folder_id);
        self.request_id = None;
        self.parent_id = parent_id;
        self.name_input.update(cx, |state, cx| {
            state.set_value(&current_name, window, cx);
        });
    }

    /// 打开对话框用于重命名请求
    pub fn open_for_request_rename(&mut self, request_id: String, current_name: String, window: &mut Window, cx: &mut Context<crate::ui::MainView>) {
        self.visible = true;
        self.is_edit = true;
        self.is_request = true;
        self.folder_id = None;
        self.request_id = Some(request_id);
        self.parent_id = None;
        self.name_input.update(cx, |state, cx| {
            state.set_value(&current_name, window, cx);
        });
    }
}

/// 渲染文件夹对话框浮层
pub fn render_folder_dialog_overlay(
    state: &Arc<Mutex<FolderDialogState>>,
    app_state: &Arc<Mutex<AppState>>,
    theme: &Theme,
    entity_id: gpui::EntityId,
    t: &dyn Fn(&str) -> SharedString,
    cx: &mut Context<crate::ui::MainView>,
) -> AnyElement {
    // 提前获取渲染所需的字段，尽早释放锁
    let (visible, is_edit, is_request, name_input) = {
        let s = state.lock().unwrap();
        (s.visible, s.is_edit, s.is_request, s.name_input.clone())
    };

    if !visible {
        return div().into_any_element();
    }

    let th = theme.clone();
    // 新建文件夹 / 重命名文件夹 / 重命名请求 共用一套版式，只换图标、标题、副标题和字段名
    let (title_text, hint_text, field_label, title_icon): (
        SharedString,
        SharedString,
        SharedString,
        IconName,
    ) = if is_request {
        (
            t("dialog.rename_request"),
            t("dialog.rename_request_hint"),
            t("dialog.request_name"),
            IconName::File,
        )
    } else if is_edit {
        (
            t("dialog.rename_folder"),
            t("dialog.rename_folder_hint"),
            t("dialog.folder_name"),
            IconName::FolderClosed,
        )
    } else {
        (
            t("dialog.new_folder"),
            t("dialog.new_folder_hint"),
            t("dialog.folder_name"),
            IconName::FolderClosed,
        )
    };

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            // 背景遮罩
            div()
                .absolute()
                .inset_0()
                .bg(th.scrim())
                .on_mouse_down(MouseButton::Left, {
                    let s = state.clone();
                    let eid = entity_id;
                    move |_, _, cx| {
                        cx.stop_propagation();
                        if let Ok(mut st) = s.lock() {
                            st.visible = false;
                        }
                        cx.notify(eid);
                    }
                }),
        )
        .child(
            // 对话框卡片（阻止事件冒泡到背景遮罩）
            div()
                .relative()
                .w(px(440.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .rounded(px(RADIUS_LG))
                .bg(th.background)
                .border(px(1.0))
                // 弹窗边框统一用主题 border 色：原来用的是 muted_background（比 border 深/浅一档），
                // 和另外几个弹窗不是一套
                .border_color(th.border)
                .shadow_2xl()
                .overflow_hidden()
                .flex_col()
                .child(
                    // 头部：图标徽章 + 标题/副标题 + 关闭
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .justify_between()
                        .gap_3()
                        .px_5()
                        .pt_5()
                        .pb_4()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_3()
                                .child(
                                    // 图标徽章：用强调色的低透明度底，视觉上比裸图标更像一个"模块"
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(36.0))
                                        .h(px(36.0))
                                        .rounded(px(RADIUS_SM))
                                        .bg(th.accent.alpha(0.12))
                                        // 36px 图标徽章属于"独立图标"档位（Regular 14px），
                                        // 副标题里说这是"文件夹"还是"请求"的强调标识 → Accent
                                        .child(themed_icon(
                                            title_icon,
                                            IconTier::Regular,
                                            IconTone::Accent,
                                            &th,
                                        )),
                                )
                                .child(
                                    div()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_base()
                                                .font_semibold()
                                                .text_color(th.foreground)
                                                .child(title_text),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(th.muted_foreground)
                                                .child(hint_text),
                                        ),
                                ),
                        )
                        .child(
                            Button::new("close-folder-dialog")
                                .icon(IconName::Close)
                                // 弹窗标题栏的关闭按钮是独立按钮 → 图标标准档 14px。
                                // 组件库 Button 的图标尺寸只由 Button 自身的 size 决定
                                // （button.rs: icon_size = size * 0.75），所以显式给该档对应的 size
                                .with_size(button_size_for_icon(IconTier::Regular))
                                // 按钮盒保持原来的 20px（组件库 XSmall 图标按钮 size_5）：
                                // 这一轮只改图标尺寸，不动按钮几何
                                .w(px(20.0))
                                .h(px(20.0))
                                .on_click({
                                    let s = state.clone();
                                    let eid = entity_id;
                                    move |_, _, cx| {
                                        if let Ok(mut st) = s.lock() {
                                            st.visible = false;
                                        }
                                        cx.notify(eid);
                                    }
                                }),
                        ),
                )
                .child(
                    // 内容区：字段名 + 输入框
                    div()
                        .px_5()
                        .pb_2()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(th.muted_foreground)
                                .child(field_label),
                        )
                        .child(
                            // 底色走 control_bg()（= input_background），全应用输入框同一个值
                            Input::new(&name_input)
                                .h(px(CONTROL_H))
                                .w_full()
                                .rounded(px(RADIUS_SM))
                                .bg(th.control_bg())
                                .text_color(th.foreground),
                        ),
                )
                .child(
                    // 底部按钮：取消（次要）+ 保存（主要，强调色）
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .px_5()
                        .pt_2()
                        .pb_5()
                        .child(
                            Button::new("cancel-folder")
                                .label(t("dialog.cancel"))
                                .outline()
                                .rounded(px(RADIUS_SM))
                                .on_click({
                                    let s = state.clone();
                                    let eid = entity_id;
                                    move |_, _, cx| {
                                        if let Ok(mut st) = s.lock() {
                                            st.visible = false;
                                        }
                                        cx.notify(eid);
                                    }
                                }),
                        )
                        .child({
                            let save_state = state.clone();
                            let save_app = app_state.clone();
                            let save_entity_id = entity_id;
                            Button::new("save-folder")
                                .label(t("dialog.save"))
                                .bg(th.accent)
                                .text_color(th.accent_foreground)
                                .rounded(px(RADIUS_SM))
                                .on_click(move |_, _, cx| {
                                    let name_input = save_state.lock().unwrap().name_input.clone();
                                    let name = name_input.read(cx).value().to_string();
                                    let trimmed = name.trim().to_string();
                                    if trimmed.is_empty() {
                                        return;
                                    }
                                    let (folder_id, parent_id, is_edit_val, is_request, request_id) = {
                                        let st = save_state.lock().unwrap();
                                        (st.folder_id.clone(), st.parent_id.clone(), st.is_edit, st.is_request, st.request_id.clone())
                                    };
                                    let app = save_app.lock().unwrap();
                                    if is_request {
                                        if let Some(ref rid) = request_id {
                                            if let Ok(mut saved) = app.db.get_saved_requests() {
                                                if let Some(req) = saved.iter_mut().find(|r| r.id == *rid) {
                                                    req.name = trimmed;
                                                    let _ = app.db.save_request(req);
                                                }
                                            }
                                        }
                                    } else if is_edit_val {
                                        if let Some(ref fid) = folder_id {
                                            let existing = app.db.get_folders().unwrap_or_default()
                                                .into_iter().find(|f| &f.id == fid);
                                            let created_at = existing.and_then(|f| f.created_at);
                                            let folder = crate::app::database::Folder {
                                                id: fid.clone(),
                                                name: trimmed,
                                                parent_id,
                                                created_at,
                                            };
                                            let _ = app.db.save_folder(&folder);
                                        }
                                    } else {
                                        match app.db.create_folder(&trimmed, parent_id.as_deref()) {
                                            Ok(f) => log::info!("文件夹已创建: id={}, name={}", f.id, f.name),
                                            Err(e) => log::error!("创建文件夹失败: {}", e),
                                        }
                                    }
                                    {
                                        let mut st = save_state.lock().unwrap();
                                        st.visible = false;
                                        st.needs_refresh = true;
                                    }
                                    cx.notify(save_entity_id);
                                })
                        }),
                ),
        )
        .into_any_element()
}
