use crate::app::database::{Folder, SavedRequest};
use crate::ui::components::{method_color, popup_panel};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::{Icon, IconName, Sizable, StyledExt};

/// 按显示宽度截断（中文≈2宽，英文≈1宽）
fn truncate_name(s: &str, max_width: usize) -> String {
    let mut w = 0;
    for (i, ch) in s.char_indices() {
        w += if ch.is_ascii() { 1 } else { 2 };
        if w > max_width {
            return format!("{}…", &s[..i]);
        }
    }
    s.to_string()
}

#[derive(Clone, Debug)]
pub enum CollectionItem {
    Folder {
        id: String,
        name: String,
        depth: usize,
        is_expanded: bool,
        children: Vec<CollectionItem>,
    },
    Request {
        id: String,
        name: String,
        method: String,
        url: String,
        depth: usize,
    },
}

fn build_tree(
    folders: &[Folder],
    requests: &[SavedRequest],
    parent_id: Option<&str>,
    expanded_ids: &std::collections::HashSet<String>,
    depth: usize,
) -> Vec<CollectionItem> {
    let mut items: Vec<CollectionItem> = Vec::new();

    let mut level_folders: Vec<&Folder> = folders
        .iter()
        .filter(|f| f.parent_id.as_deref() == parent_id)
        .collect();
    level_folders.sort_by(|a, b| a.name.cmp(&b.name));

    for folder in level_folders {
        let is_expanded = expanded_ids.contains(&folder.id);
        let children = if is_expanded {
            build_tree(folders, requests, Some(&folder.id), expanded_ids, depth + 1)
        } else {
            Vec::new()
        };
        items.push(CollectionItem::Folder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            depth,
            is_expanded,
            children,
        });
    }

    let mut level_requests: Vec<&SavedRequest> = requests
        .iter()
        .filter(|r| r.folder_id.as_deref() == parent_id)
        .collect();
    level_requests.sort_by_key(|r| std::cmp::Reverse(r.updated_at));

    let req_depth = if parent_id.is_none() { depth } else { depth + 1 };
    for req in level_requests {
        items.push(CollectionItem::Request {
            id: req.id.clone(),
            name: req.name.clone(),
            method: req.method.clone(),
            url: req.url.clone(),
            depth: req_depth,
        });
    }

    items
}

pub fn build_collection_tree(
    folders: &[Folder],
    requests: &[SavedRequest],
    expanded_ids: &std::collections::HashSet<String>,
) -> Vec<CollectionItem> {
    build_tree(folders, requests, None, expanded_ids, 0)
}

pub fn render_collection_panel(
    items: &[CollectionItem],
    context_menu_target: &Option<String>,
    hovered_item_name: &Option<String>,
    cx: &mut Context<MainView>,
    theme: &Theme,
) -> impl IntoElement {
    let menu_target = context_menu_target.clone();

    let mut all_items: Vec<gpui::AnyElement> = Vec::new();
    for item in items {
        let elem = match item {
            CollectionItem::Folder { id, name, depth, is_expanded, children } => {
                let fid = id.clone();
                let fname = name.clone();
                let expanded = *is_expanded;
                let d = *depth;
                let child_items = children.clone();
                let menu_open = menu_target.as_ref() == Some(&fid);

                div()
                    .w_full()
                    .flex_col()
                    .children([
                        div()
                            .w_full()
                            .relative()
                            .flex()
                            .flex_row()
                            .items_center()
                            .py_1()
                            .px_1()
                            .gap(px(2.0))
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.code_background))
                            .children([
                                div().w(px(d as f32 * 16.0)).flex_shrink_0().into_any_element(),
                                div()
                                    .w(px(16.0)).h(px(16.0))
                                    .flex().items_center().justify_center()
                                    .flex_shrink_0()
                                    .on_mouse_down(MouseButton::Left, {
                                        let toggle_id = fid.clone();
                                        cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            this.toggle_folder_expand(&toggle_id, cx);
                                        })
                                    })
                                    .child(if expanded {
                                        Icon::new(IconName::ChevronDown).xsmall().text_color(theme.muted_foreground)
                                    } else {
                                        Icon::new(IconName::ChevronRight).xsmall().text_color(theme.muted_foreground)
                                    })
                                    .into_any_element(),
                                div()
                                    .flex_shrink_0()
                                    .child(
                                        Icon::new(if expanded { IconName::FolderOpen } else { IconName::FolderClosed })
                                            .xsmall().text_color(theme.accent),
                                    )
                                    .into_any_element(),
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .on_mouse_move({
                                        let full_name = fname.clone();
                                        cx.listener(move |this, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            cx.stop_propagation();
                                            this.hovered_item_name = Some(full_name.clone());
                                            this.hovered_item_y = Some(e.position.y.into());
                                            this.hovered_item_x = Some(e.position.x.into());
                                            cx.notify();
                                        })
                                    })
                                    .child(truncate_name(&fname, 24usize.saturating_sub(d * 2)))
                                    .into_any_element(),
                                div()
                                    .flex_shrink_0()
                                    .w(px(24.0)).h(px(24.0))
                                    .flex().items_center().justify_center()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.muted_background))
                                    .on_mouse_down(MouseButton::Left, {
                                        let target_id = fid.clone();
                                        cx.listener(move |this, e: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            cx.stop_propagation();
                                            this.context_menu_pos = Some((e.position.x.into(), e.position.y.into()));
                                            this.context_menu_target = if this.context_menu_target.as_ref() == Some(&target_id) {
                                                None
                                            } else {
                                                Some(target_id.clone())
                                            };
                                            cx.notify();
                                        })
                                    })
                                    .child(Icon::new(IconName::Ellipsis).xsmall().text_color(theme.muted_foreground))
                                    .into_any_element(),
                                div().into_any_element(),
                            ])
                            .into_any_element(),
                        if expanded && !child_items.is_empty() {
                            div().child(render_collection_panel(&child_items, context_menu_target, hovered_item_name, cx, theme)).into_any_element()
                        } else {
                            div().into_any_element()
                        },
                    ])
                    .into_any_element()
            }
            CollectionItem::Request { id, name, method, url, depth } => {
                let rid = id.clone();
                let rname = name.clone();
                let rmethod = method.clone();
                let rurl = url.clone();
                let d = *depth;
                let method_clr = method_color(&rmethod);
                let menu_open = menu_target.as_ref() == Some(&rid);

                div()
                    .w_full()
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .py_1()
                    .px_1()
                    .gap(px(2.0))
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.code_background))
                    .children([
                        div().w(px(d as f32 * 16.0)).flex_shrink_0().into_any_element(),
                        div().w(px(16.0)).flex_shrink_0().into_any_element(),
                        div()
                            .px_1().py_px()
                            .rounded_sm()
                            .bg(rgb(method_clr))
                            .text_xs().text_color(rgb(0xffffff))
                            .flex_shrink_0()
                            .child(rmethod.clone())
                            .into_any_element(),
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .on_mouse_move({
                                let full_name = rname.clone();
                                cx.listener(move |this, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    cx.stop_propagation();
                                    this.hovered_item_name = Some(full_name.clone());
                                    this.hovered_item_y = Some(e.position.y.into());
                                    this.hovered_item_x = Some(e.position.x.into());
                                    cx.notify();
                                })
                            })
                            .on_mouse_down(MouseButton::Left, {
                                let req_id = rid.clone();
                                let req_method = rmethod.clone();
                                let req_url = rurl.clone();
                                let req_name = rname.clone();
                                cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    this.load_saved_request_by_id(&req_id, &req_method, &req_url, &req_name, _window, cx);
                                })
                            })
                            .child(truncate_name(&rname, 24usize.saturating_sub(d * 2)))
                            .into_any_element(),
                        div()
                            .flex_shrink_0()
                            .w(px(24.0)).h(px(24.0))
                            .flex().items_center().justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.muted_background))
                            .on_mouse_down(MouseButton::Left, {
                                let target_id = rid.clone();
                                cx.listener(move |this, e: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    cx.stop_propagation();
                                    this.context_menu_pos = Some((e.position.x.into(), e.position.y.into()));
                                    this.context_menu_target = if this.context_menu_target.as_ref() == Some(&target_id) {
                                        None
                                    } else {
                                        Some(target_id.clone())
                                    };
                                    cx.notify();
                                })
                            })
                            .child(Icon::new(IconName::Ellipsis).xsmall().text_color(theme.muted_foreground))
                            .into_any_element(),
                        div().into_any_element(),
                    ])
                    .into_any_element()
            }
        };
        all_items.push(elem);
    }

    div()
        .w_full()
        .relative()
        .flex_col()
        .children(all_items)
}

pub fn render_folder_context_menu(
    folder_id: &str,
    folder_name: &str,
    cx: &mut Context<MainView>,
    theme: &Theme,
) -> gpui::Div {
    let fid = folder_id.to_string();
    let fname = folder_name.to_string();

    popup_panel(theme)
        .min_w(px(140.0))
        .child(menu_item("重命名", IconName::Replace, theme, cx, {
            let edit_id = fid.clone();
            let edit_name = fname.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_folder_edit_dialog(edit_id.clone(), edit_name.clone(), window, cx);
                cx.notify();
            }
        }))
        .child(menu_item("移动到...", IconName::ArrowRight, theme, cx, {
            let move_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_move_dialog(&move_id, true, window, cx);
                cx.notify();
            }
        }))
        .child(menu_item("添加子文件夹", IconName::Plus, theme, cx, {
            let parent_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_folder_dialog(Some(parent_id.clone()), window, cx);
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item_danger("删除", IconName::Delete, theme, cx, {
            let del_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.delete_folder_from_sidebar(&del_id, window, cx);
                cx.notify();
            }
        }))
}

pub fn render_request_context_menu(
    request_id: &str,
    cx: &mut Context<MainView>,
    theme: &Theme,
) -> gpui::Div {
    let rid = request_id.to_string();

    popup_panel(theme)
        .min_w(px(120.0))
        .child(menu_item("移动到...", IconName::ArrowRight, theme, cx, {
            let move_id = rid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_move_dialog(&move_id, false, window, cx);
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item_danger("删除", IconName::Delete, theme, cx, {
            let del_id = rid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.delete_saved_request_from_sidebar(&del_id, window, cx);
                cx.notify();
            }
        }))
}

fn menu_item(
    label: &str,
    icon: IconName,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_click: impl Fn(&mut MainView, &MouseDownEvent, &mut Window, &mut Context<MainView>) + 'static,
) -> impl IntoElement {
    div()
        .flex().flex_row().items_center().gap_2()
        .px_3().py_1p5()
        .cursor_pointer()
        .hover(|s| s.bg(theme.muted_background))
        .on_mouse_down(MouseButton::Left, cx.listener(on_click))
        .child(Icon::new(icon).xsmall().text_color(theme.muted_foreground))
        .child(div().text_sm().text_color(theme.foreground).child(label.to_string()))
}

fn menu_item_danger(
    label: &str,
    icon: IconName,
    theme: &Theme,
    cx: &mut Context<MainView>,
    on_click: impl Fn(&mut MainView, &MouseDownEvent, &mut Window, &mut Context<MainView>) + 'static,
) -> impl IntoElement {
    div()
        .flex().flex_row().items_center().gap_2()
        .px_3().py_1p5()
        .cursor_pointer()
        .hover(|s| s.bg(theme.error))
        .on_mouse_down(MouseButton::Left, cx.listener(on_click))
        .child(Icon::new(icon).xsmall().text_color(theme.muted_foreground))
        .child(div().text_sm().text_color(theme.foreground).child(label.to_string()))
}
