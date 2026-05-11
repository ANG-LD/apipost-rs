/// 文件夹创建/重命名对话框
use crate::app::AppState;
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, Icon, IconName, Sizable, StyledExt};
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

    let t = theme.clone();
    let (title_text, title_icon): (&str, IconName) = if is_request {
        ("重命名请求", IconName::File)
    } else if is_edit {
        ("重命名文件夹", IconName::FolderClosed)
    } else {
        ("新建文件夹", IconName::FolderClosed)
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
                .bg(rgba(0x00000044))
                .on_mouse_down(MouseButton::Left, {
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
        .child(
            // 对话框卡片（阻止事件冒泡到背景遮罩）
            div()
                .relative()
                .w(px(420.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .rounded_lg()
                .bg(t.background)
                .border(px(1.0))
                .border_color(t.muted_background)
                .shadow_lg()
                .flex_col()
                .child(
                    // 标题栏
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_3()
                        .border_b(px(1.0))
                        .border_color(t.muted_background)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(title_icon).small().text_color(t.accent))
                                .child(div().font_semibold().text_color(t.foreground).child(title_text)),
                        )
                        .child(
                            Button::new("close-folder-dialog")
                                .icon(IconName::Close)
                                .xsmall()
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
                    // 内容区
                    div()
                        .px_4()
                        .py_4()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(t.muted_foreground)
                                .child("文件夹名称"),
                        )
                        .child(
                            Input::new(&name_input)
                                .h(px(38.0))
                                .w_full()
                                .bg(t.background)
                                .text_color(t.foreground),
                        ),
                )
                .child(
                    // 底部按钮
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .px_4()
                        .py_3()
                        .border_t(px(1.0))
                        .border_color(t.muted_background)
                        .child(
                            Button::new("cancel-folder")
                                .label("取消")
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
                                .label("保存")
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
