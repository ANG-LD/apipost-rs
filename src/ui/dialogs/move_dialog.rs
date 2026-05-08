/// 移动请求/文件夹到指定文件夹对话框
use crate::app::database::Folder;
use crate::app::AppState;
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use std::sync::{Arc, Mutex};

/// 移动对话框状态
pub struct MoveDialogState {
    pub visible: bool,
    /// 待移动的项目 ID
    pub item_id: String,
    /// true = 移动文件夹, false = 移动请求
    pub is_folder: bool,
    /// 选中的目标文件夹 ID (None = 根目录)
    pub selected_folder_id: Option<String>,
    pub needs_refresh: bool,
}

impl MoveDialogState {
    pub fn new() -> Self {
        Self {
            visible: false,
            item_id: String::new(),
            is_folder: false,
            selected_folder_id: None,
            needs_refresh: false,
        }
    }

    pub fn open(&mut self, item_id: String, is_folder: bool) {
        self.visible = true;
        self.item_id = item_id;
        self.is_folder = is_folder;
        self.selected_folder_id = None;
    }
}

/// 计算文件夹深度（用于缩进显示）
fn folder_depth(folders: &[Folder], folder_id: &str) -> usize {
    let folder = match folders.iter().find(|f| f.id == folder_id) {
        Some(f) => f,
        None => return 0,
    };
    match &folder.parent_id {
        Some(pid) => 1 + folder_depth(folders, pid),
        None => 0,
    }
}

/// 收集文件夹的所有子孙 ID
fn collect_descendant_ids(folders: &[Folder], folder_id: &str) -> Vec<String> {
    let mut result = Vec::new();
    for f in folders {
        if f.parent_id.as_deref() == Some(folder_id) {
            result.push(f.id.clone());
            result.extend(collect_descendant_ids(folders, &f.id));
        }
    }
    result
}

/// 渲染移动文件夹选择浮层
pub fn render_move_dialog_overlay(
    state: &Arc<Mutex<MoveDialogState>>,
    app_state: &Arc<Mutex<AppState>>,
    theme: &Theme,
    entity_id: gpui::EntityId,
    cx: &mut Context<crate::ui::MainView>,
) -> AnyElement {
    // 提前获取渲染所需字段，尽早释放锁
    let (visible, item_id, is_folder, selected) = {
        let s = state.lock().unwrap();
        if !s.visible {
            return div().into_any_element();
        }
        (true, s.item_id.clone(), s.is_folder, s.selected_folder_id.clone())
    };

    let t = theme.clone();
    let folders: Vec<Folder> = app_state.lock().unwrap().db.get_folders().unwrap_or_default();

    // 过滤掉自身及子孙（文件夹不能移动到自己的子文件夹里）
    let descendants = if is_folder {
        collect_descendant_ids(&folders, &item_id)
    } else {
        Vec::new()
    };
    let target_folders: Vec<&Folder> = folders
        .iter()
        .filter(|f| {
            if is_folder {
                f.id != item_id && !descendants.iter().any(|d| d == &f.id)
            } else {
                true
            }
        })
        .collect();

    // 按深度排序，同级按名称排序
    let mut sorted: Vec<&Folder> = target_folders;
    sorted.sort_by(|a, b| {
        let a_depth = folder_depth(&folders, &a.id);
        let b_depth = folder_depth(&folders, &b.id);
        a_depth.cmp(&b_depth).then(a.name.cmp(&b.name))
    });

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
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
                .max_h(px(500.0))
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
                    // 标题
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
                                .child(Icon::new(IconName::FolderOpen).small().text_color(t.accent))
                                .child(div().font_semibold().text_color(t.foreground).child("移动到...")),
                        )
                        .child(
                            Button::new("close-move-dialog")
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
                    // 文件夹列表
                    div()
                        .id("move-folder-list")
                        .flex_1()
                        .flex_col()
                        .overflow_y_scroll()
                        .max_h(px(360.0))
                        .child(
                            // "根目录" 选项
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_4()
                                .py_2()
                                .cursor_pointer()
                                .hover(|s| s.bg(t.code_background))
                                .bg(if selected.is_none() { t.muted_background } else { t.background })
                                .child(Icon::new(IconName::FolderClosed).xsmall().text_color(t.muted_foreground))
                                .child(div().text_sm().text_color(t.foreground).child("根目录 (无文件夹)"))
                                .on_mouse_down(MouseButton::Left, {
                                    let s = state.clone();
                                    let eid = entity_id;
                                    move |_, _, cx| {
                                        if let Ok(mut st) = s.lock() {
                                            st.selected_folder_id = None;
                                        }
                                        cx.notify(eid);
                                    }
                                }),
                        )
                        .children(sorted.iter().map(|f| {
                            let fid = f.id.clone();
                            let fname = f.name.clone();
                            let depth = folder_depth(&folders, &fid);
                            let is_selected = selected.as_deref() == Some(&fid);
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_4()
                                .py_2()
                                .cursor_pointer()
                                .hover(|s| s.bg(t.code_background))
                                .bg(if is_selected { t.muted_background } else { t.background })
                                .child(div().w(px(depth as f32 * 20.0)))
                                .child(Icon::new(IconName::FolderClosed).xsmall().text_color(t.accent))
                                .child(div().text_sm().text_color(t.foreground).child(fname.clone()))
                                .on_mouse_down(MouseButton::Left, {
                                    let s = state.clone();
                                    let fid_clone = fid.clone();
                                    let eid = entity_id;
                                    move |_, _, cx| {
                                        if let Ok(mut st) = s.lock() {
                                            st.selected_folder_id = Some(fid_clone.clone());
                                        }
                                        cx.notify(eid);
                                    }
                                })
                        })),
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
                            Button::new("cancel-move")
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
                            let save_eid = entity_id;
                            Button::new("confirm-move")
                                .label("移动")
                                .on_click(move |_, _, cx| {
                                    let (selected_folder, move_item_id, move_is_folder) = {
                                        let st = save_state.lock().unwrap();
                                        (st.selected_folder_id.clone(), st.item_id.clone(), st.is_folder)
                                    };
                                    let app = save_app.lock().unwrap();
                                    if move_is_folder {
                                        let _ = app.db.move_folder(&move_item_id, selected_folder.as_deref());
                                    } else {
                                        let _ = app.db.move_request(&move_item_id, selected_folder.as_deref());
                                    }
                                    if let Ok(mut st) = save_state.lock() {
                                        st.visible = false;
                                        st.needs_refresh = true;
                                    }
                                    cx.notify(save_eid);
                                })
                        }),
                ),
        )
        .into_any_element()
}
