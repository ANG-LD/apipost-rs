use crate::app::database::{Folder, SavedRequest};
use crate::app::AppState;
use crate::http::{generate_curl, HttpRequest};
use crate::ui::clipboard;
use crate::ui::components::{method_color, popup_panel};
use crate::ui::main_view::MainView;
use crate::ui::Theme;
use gpui::*;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use std::sync::{Arc, Mutex};

/// 拖拽数据
#[derive(Clone)]
pub struct DragItem {
    pub id: String,
    pub is_folder: bool,
    pub name: String,
}

/// 拖拽预览（跟随鼠标显示）
struct DragPreview {
    label: gpui::SharedString,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgba(0x000000cc))
            .text_color(rgb(0xffffff))
            .text_sm()
            .child(self.label.clone())
    }
}

/// 按显示宽度截断（中文≈2宽，英文≈1宽）
fn truncate_name(s: &str, max_width: usize) -> std::borrow::Cow<'_, str> {
    let mut w = 0;
    for (i, ch) in s.char_indices() {
        w += if ch.is_ascii() { 1 } else { 2 };
        if w > max_width {
            return std::borrow::Cow::Owned(format!("{}…", &s[..i]));
        }
    }
    std::borrow::Cow::Borrowed(s)
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
    app_state: &Arc<Mutex<AppState>>,
    needs_drop_refresh: Arc<std::sync::atomic::AtomicBool>,
) -> impl IntoElement {
    let menu_target = context_menu_target.clone();
    let entity_id = cx.entity_id();

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

                let drag_item = DragItem { id: fid.clone(), is_folder: true, name: fname.clone() };
                let target_fid = fid.clone();
                let drop_app = app_state.clone();
                let drop_flag = needs_drop_refresh.clone();
                let drop_eid = entity_id;

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
                            .id(ElementId::from(format!("folder-{}", fid)))
                            .on_drag(drag_item, |data: &DragItem, _offset, window, cx| {
                                cx.new(|_| DragPreview { label: data.name.clone().into() })
                            })
                            .drag_over::<DragItem>(|style, _data, _window, _cx| {
                                style.bg(rgba(0x88888844))
                            })
                            .on_drop::<DragItem>(move |data: &DragItem, _window, cx| {
                                if data.id == target_fid { return; }
                                if let Ok(app) = drop_app.lock() {
                                    if data.is_folder {
                                        let _ = app.db.move_folder(&data.id, Some(&target_fid));
                                    } else {
                                        let _ = app.db.move_request(&data.id, Some(&target_fid));
                                    }
                                }
                                drop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                                cx.notify(drop_eid);
                            })
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
                                    .child(truncate_name(&fname, 24usize.saturating_sub(d * 2)).into_owned())
                                    .into_any_element(),
                                div().into_any_element(),
                            ])
                            // ...按钮绝对定位固定右侧
                            .child(
                                div()
                                    .absolute()
                                    .right(px(0.0))
                                    .top(px(0.0))
                                    .h_full()
                                    .flex().items_center()
                                    .bg(theme.background)
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                    .child(
                                        div()
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
                                            .child(Icon::new(IconName::Ellipsis).xsmall().text_color(theme.foreground)),
                                    ),
                            )
                            .child(div().into_any_element())
                            .into_any_element(),
                        if expanded && !child_items.is_empty() {
                            div().child(render_collection_panel(&child_items, context_menu_target, hovered_item_name, cx, theme, app_state, needs_drop_refresh.clone())).into_any_element()
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
                let req_drag = DragItem { id: rid.clone(), is_folder: false, name: rname.clone() };

                div()
                    .w_full()
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .py_1()
                    .px_1()
                    .pr(px(30.0))
                    .gap(px(2.0))
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.code_background))
                    .id(ElementId::from(format!("req-{}", rid)))
                    .on_drag(req_drag, |data: &DragItem, _offset, window, cx| {
                        cx.new(|_| DragPreview { label: data.name.clone().into() })
                    })
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
                            .child(truncate_name(&rname, 24usize.saturating_sub(d * 2)).into_owned())
                            .into_any_element(),
                        div().into_any_element(),
                    ])
                    .child(
                        div()
                            .absolute()
                            .right(px(0.0))
                            .top(px(0.0))
                            .h_full()
                            .flex().items_center()
                            .bg(theme.background)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                div()
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
                                    .child(Icon::new(IconName::Ellipsis).xsmall().text_color(theme.foreground)),
                            ),
                    )
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
    t: &dyn Fn(&str) -> String,
) -> gpui::Div {
    let fid = folder_id.to_string();
    let fname = folder_name.to_string();

    popup_panel(theme)
        .min_w(px(140.0))
        .child(menu_item(&t("context.rename"), IconName::Replace, theme, cx, {
            let edit_id = fid.clone();
            let edit_name = fname.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_folder_edit_dialog(edit_id.clone(), edit_name.clone(), window, cx);
                cx.notify();
            }
        }))
        .child(menu_item(&t("context.move_to"), IconName::ArrowRight, theme, cx, {
            let move_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_move_dialog(&move_id, true, window, cx);
                cx.notify();
            }
        }))
        .child(menu_item(&t("context.add_subfolder"), IconName::Plus, theme, cx, {
            let parent_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_folder_dialog(Some(parent_id.clone()), window, cx);
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item_danger(&t("context.delete"), IconName::Delete, theme, cx, {
            let del_id = fid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.delete_folder_from_sidebar(&del_id, window, cx);
                cx.notify();
            }
        }))
}

pub fn render_request_context_menu(
    request: &SavedRequest,
    cx: &mut Context<MainView>,
    theme: &Theme,
    t: &dyn Fn(&str) -> String,
) -> gpui::Div {
    let rid = request.id.clone();
    let rname = request.name.clone();

    // Build HttpRequest — headers 存储格式为 "Key: Value\n" 纯文本
    let parsed_headers: Vec<(String, String)> = request
        .headers
        .as_ref()
        .map(|h| {
            h.lines()
                .filter_map(|line| {
                    let mut parts = line.splitn(2, ':');
                    let key = parts.next()?.trim();
                    let value = parts.next()?.trim();
                    if key.is_empty() { None } else { Some((key.to_string(), value.to_string())) }
                })
                .collect()
        })
        .unwrap_or_default();

    let http_request = HttpRequest {
        method: request.method.clone(),
        url: request.url.clone(),
        headers: parsed_headers,
        body: request.body.clone(),
        content_type: None,
        text_fields: Vec::new(),
        file_fields: Vec::new(),
    };
    let curl_cmd = generate_curl(&http_request);
    let headers_json = request.headers.clone().unwrap_or_default();
    let body_text = request.body.clone().unwrap_or_default();

    popup_panel(theme)
        .min_w(px(180.0))
        .child(menu_item(&t("context.copy_curl"), IconName::Copy, theme, cx, {
            let cmd = curl_cmd.clone();
            move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                clipboard::copy_to_clipboard(&cmd);
                cx.notify();
            }
        }))
        .child(menu_item(&t("context.generate_code"), IconName::File, theme, cx, {
            let req_data = http_request.clone();
            move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.code_gen_dialog_state
                    .lock()
                    .unwrap()
                    .open_dialog(req_data.clone());
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item(&t("context.copy_body"), IconName::Copy, theme, cx, {
            let body = body_text.clone();
            move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                if !body.is_empty() {
                    clipboard::copy_to_clipboard(&body);
                }
                cx.notify();
            }
        }))
        .child(menu_item(&t("context.copy_headers"), IconName::File, theme, cx, {
            let headers = headers_json.clone();
            move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                if !headers.is_empty() {
                    clipboard::copy_to_clipboard(&headers);
                }
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item(&t("context.share_request"), IconName::Check, theme, cx, {
            let req_data = http_request.clone();
            move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                if let Ok(json) = serde_json::to_string_pretty(&req_data) {
                    clipboard::copy_to_clipboard(&json);
                }
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item(&t("context.rename"), IconName::Replace, theme, cx, {
            let rename_id = rid.clone();
            let rename_name = rname.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_request_rename_dialog(&rename_id, &rename_name, window, cx);
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item(&t("context.move_to"), IconName::ArrowRight, theme, cx, {
            let move_id = rid.clone();
            move |this, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<MainView>| {
                this.context_menu_target = None;
                this.open_move_dialog(&move_id, false, window, cx);
                cx.notify();
            }
        }))
        .child(div().w_full().h(px(1.0)).bg(theme.muted_background))
        .child(menu_item_danger(&t("context.delete"), IconName::Delete, theme, cx, {
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
