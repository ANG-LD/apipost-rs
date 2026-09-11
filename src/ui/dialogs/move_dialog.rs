/// 移动请求/文件夹到指定文件夹对话框
use crate::app::database::Folder;
use crate::app::AppState;
use crate::ui::components::{
    button_size_for_icon, themed_icon, IconTier, IconTone, ICON_TEXT_GAP, RADIUS_LG,
};
use crate::ui::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::{IconName, Sizable, StyledExt};
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
                .bg(t.scrim())
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
                .w(px(420.0))
                .max_h(px(500.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .rounded(px(RADIUS_LG))
                .bg(t.background)
                .border(px(1.0))
                // 边框/阴影与其它弹窗统一
                .border_color(t.border)
                .shadow_2xl()
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
                        .border_color(t.border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(ICON_TEXT_GAP))
                                // 弹窗标题图标：与 14px 半粗标题同档（Regular），
                                // "移动目标"是文件夹语义 → Accent
                                .child(themed_icon(
                                    IconName::FolderOpen,
                                    IconTier::Regular,
                                    IconTone::Accent,
                                    &t,
                                ))
                                .child(div().font_semibold().text_color(t.foreground).child("移动到...")),
                        )
                        .child(
                            Button::new("close-move-dialog")
                                .icon(IconName::Close)
                                // 同 folder_dialog 的关闭按钮：独立按钮 → 图标标准档 14px
                                .with_size(button_size_for_icon(IconTier::Regular))
                                // 按钮盒保持原来的 20px（组件库 XSmall 图标按钮 size_5）
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
                                // 可点击行必须给 id，hover 样式才会真的参与计算
                                .id("move-dialog-root-row")
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(ICON_TEXT_GAP))
                                .px_4()
                                .py_2()
                                .cursor_pointer()
                                // hover 统一走 tokens：常态行提亮一档，既然它当前是选中态（已经是 muted 面）
                                // 就再压深一档，否则 hover 与底色同色、看不出反馈
                                .hover(|s| {
                                    if selected.is_none() {
                                        s.bg(t.active_bg())
                                    } else {
                                        s.bg(t.hover_bg())
                                    }
                                })
                                // 按下统一走 active_bg()：不许用 .opacity() 整元素变透明
                                .active(|s| s.bg(t.active_bg()))
                                .bg(if selected.is_none() { t.muted_background } else { t.background })
                                // "根目录(无文件夹)" 是个次级占位项 → Muted
                                .child(themed_icon(
                                    IconName::FolderClosed,
                                    IconTier::Dense,
                                    IconTone::Muted,
                                    &t,
                                ))
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
                                // 列表行在循环里生成，id 必须带 fid 才唯一
                                // （共用 id 会让所有行共享同一份 hover 状态）
                                .id(ElementId::from(format!("move-dialog-folder-{}", fid)))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(ICON_TEXT_GAP))
                                .px_4()
                                .py_2()
                                .cursor_pointer()
                                .hover(|s| {
                                    if is_selected {
                                        s.bg(t.active_bg())
                                    } else {
                                        s.bg(t.hover_bg())
                                    }
                                })
                                // 按下统一走 active_bg()
                                .active(|s| s.bg(t.active_bg()))
                                .bg(if is_selected { t.muted_background } else { t.background })
                                .child(div().w(px(depth as f32 * 20.0)))
                                // 真实文件夹 → 与侧栏文件夹树、folder_dialog 同色（Accent）
                                .child(themed_icon(
                                    IconName::FolderClosed,
                                    IconTier::Dense,
                                    IconTone::Accent,
                                    &t,
                                ))
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
                        .border_color(t.border)
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
