use crate::app::database::{Folder, SavedRequest};
use crate::app::AppState;
use crate::http::{generate_curl, HttpRequest};
use crate::ui::clipboard;
use crate::ui::components::{
    icon_button, method_color, popup_panel, themed_icon, IconTier, IconTone, ICON_TEXT_GAP,
};
use crate::ui::main_view::MainView;
use crate::ui::components::{RADIUS_SM, RADIUS_XS};
use crate::ui::Theme;
use gpui::*;
use gpui_component::{IconName, StyledExt};
use std::sync::{Arc, Mutex};

/// 拖拽数据
///
/// `Arc<str>` 而不是 `String`：这个结构体每帧每个条目都要建一个（`on_drag` 要 `'static`），
/// 用 `String` 的话每帧每行都多两次堆分配；`Arc<str>` 的 clone 只是引用计数 +1，
/// 底层字符串在树里只有一份。
#[derive(Clone)]
pub struct DragItem {
    pub id: Arc<str>,
    pub is_folder: bool,
    pub name: Arc<str>,
}

/// 拖拽预览（跟随鼠标显示）
///
/// 配色由调用方从主题取好再传进来（这里没有 theme 上下文）：
/// 用和上下文菜单/悬浮提示同一套浮层配色（主题底色 + 边框 + 前景），
/// 不再写死 rgba(0,0,0,0.8) + 白色文字。
struct DragPreview {
    label: gpui::SharedString,
    bg: Rgba,
    border: Rgba,
    fg: Rgba,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded(px(RADIUS_XS))
            .border_1()
            .border_color(self.border)
            .bg(self.bg)
            .text_color(self.fg)
            .text_sm()
            .shadow_lg()
            .child(self.label.clone())
    }
}

/// 按显示宽度截断（中文≈2宽，英文≈1宽）
///
/// 返回 `Cow`：没超宽时零分配借用原串（这也是 `pub(crate)` 给性能测试用的原因）。
pub(crate) fn truncate_name(s: &str, max_width: usize) -> std::borrow::Cow<'_, str> {
    let mut w = 0;
    for (i, ch) in s.char_indices() {
        w += if ch.is_ascii() { 1 } else { 2 };
        if w > max_width {
            return std::borrow::Cow::Owned(format!("{}…", &s[..i]));
        }
    }
    std::borrow::Cow::Borrowed(s)
}

/// 渲染一个条目每帧都要用到的派生字符串：**建树时算一次，渲染时只做引用计数**。
///
/// 这些值只跟 `id` / `name` / `depth` 有关，树不变就不会变，所以完全可以缓存。
/// 改造前它们是渲染里现算的：每个条目每帧 2 个 `format!`（element id）+ 1 个
/// `format!`（更多按钮 id）+ 1 次 `truncate_name().into_owned()`，
/// 一个 56 项的收藏夹每帧就是 200+ 次堆分配（且默认侧栏标签页就是收藏夹）。
#[derive(Clone, Debug)]
pub struct ItemKeys {
    /// 行本身的 element id（hover/active 状态挂在它上面，必须按条目唯一）
    pub row: SharedString,
    /// 展开箭头按钮的 element id（请求条目没有展开箭头，留空）
    pub toggle: SharedString,
    /// 「更多…」按钮的 element id
    pub more: SharedString,
    /// 列表里显示的名字（已按层级截断）
    pub label: SharedString,
}

#[derive(Clone, Debug)]
pub enum CollectionItem {
    Folder {
        /// `Arc<str>` 而非 `String`：字段每帧都要 clone 进 `'static` 闭包，
        /// `String` 的 clone 是堆分配，`Arc<str>` 只是引用计数 +1
        id: Arc<str>,
        name: Arc<str>,
        depth: usize,
        is_expanded: bool,
        /// `Arc<[CollectionItem]>` 而非 `Vec`：整棵子树每帧 clone 一次，
        /// `Vec` 会连同每个子节点的 String 一起深拷贝，`Arc<[..]>` 只是一次引用计数。
        ///
        /// 取舍：`Arc<[T]>` 失去了 `Arc::make_mut` 的廉价原地写路径（`Arc<Vec<T>>` 有）。
        /// 这里换得划算 —— 树是**写一次、读每帧**：改动展开状态时整棵树重新构建
        /// （`build_collection_tree`），从不原地改，所以用不上 `make_mut`。
        children: Arc<[CollectionItem]>,
        keys: ItemKeys,
    },
    Request {
        id: Arc<str>,
        name: Arc<str>,
        method: Arc<str>,
        url: Arc<str>,
        depth: usize,
        keys: ItemKeys,
    },
}

/// 生成一个条目的全部缓存键：与改造前渲染里现算的写法逐字一致
/// （`folder-{id}` / `req-{id}` / `folder-toggle-{id}` / `more-{id}`）。
fn item_keys(kind: ItemKind, id: &str, name: &str, depth: usize) -> ItemKeys {
    let (row_prefix, toggle) = match kind {
        ItemKind::Folder => ("folder", format!("folder-toggle-{id}")),
        ItemKind::Request => ("req", String::new()),
    };
    ItemKeys {
        row: SharedString::from(format!("{row_prefix}-{id}")),
        toggle: SharedString::from(toggle),
        more: SharedString::from(format!("more-{id}")),
        // 显示宽度截断：与改造前 `truncate_name(&name, 24 - depth*2).into_owned()` 同式
        label: SharedString::from(
            truncate_name(name, 24usize.saturating_sub(depth * 2)).into_owned(),
        ),
    }
}

#[derive(Clone, Copy)]
enum ItemKind {
    Folder,
    Request,
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
            // 建树时把 String 转成 Arc<str>：这里是「释放一次」（每次刷新才跑），
            // 换来渲染每帧的 clone 全部变成引用计数
            id: Arc::from(folder.id.as_str()),
            name: Arc::from(folder.name.as_str()),
            depth,
            is_expanded,
            children: Arc::from(children),
            keys: item_keys(ItemKind::Folder, &folder.id, &folder.name, depth),
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
            id: Arc::from(req.id.as_str()),
            name: Arc::from(req.name.as_str()),
            method: Arc::from(req.method.as_str()),
            url: Arc::from(req.url.as_str()),
            depth: req_depth,
            keys: item_keys(ItemKind::Request, &req.id, &req.name, req_depth),
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

/// 注：曾经还有 `context_menu_target` / `hovered_item_name` 两个参数，
/// 但函数体里从未读过它们（只往递归里转发），是纯死参数 —— 一并删掉。
/// 上下文菜单和高亮名由 main_view 自己按 `context_menu_target` 渲染。
pub fn render_collection_panel(
    items: &[CollectionItem],
    cx: &mut Context<MainView>,
    theme: &Theme,
    app_state: &Arc<Mutex<AppState>>,
    needs_drop_refresh: Arc<std::sync::atomic::AtomicBool>,
) -> impl IntoElement {
    let entity_id = cx.entity_id();
    // 浮层（拖拽预览）与落点高亮需要的颜色：闭包/子视图要求 'static 或没有 theme 上下文，
    // Rgba 是 Copy，先取出来最省事，也避免每行 clone 整个 Theme
    let drag_bg = theme.background;
    let drag_border = theme.border;
    let drag_fg = theme.foreground;
    let drop_tint = theme.accent.alpha(0.12);

    let mut all_items: Vec<gpui::AnyElement> = Vec::new();
    for item in items {
        let elem = match item {
            CollectionItem::Folder { id, name, depth, is_expanded, children, keys } => {
                // 全部是 Arc/SharedString 的引用计数克隆，不再每帧复制字符串
                let fid = Arc::clone(id);
                let fname = Arc::clone(name);
                let expanded = *is_expanded;
                let d = *depth;
                let child_items = Arc::clone(children);
                // 注：原先这里还有个 menu_open（拿 menu_target 现算），但它从未被使用；
                // 顺带删掉了每帧一次 context_menu_target.clone()

                let drag_item = DragItem { id: Arc::clone(&fid), is_folder: true, name: Arc::clone(&fname) };
                let target_fid = Arc::clone(&fid);
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
                            .rounded(px(RADIUS_SM))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover_bg()))
                            .id(ElementId::Name(keys.row.clone()))
                            .on_drag(drag_item, move |data: &DragItem, _offset, window, cx| {
                                cx.new(|_| DragPreview { label: data.name.clone().into(), bg: drag_bg, border: drag_border, fg: drag_fg })
                            })
                            .drag_over::<DragItem>(move |style, _data, _window, _cx| {
                                // 落点高亮用主题主色淡化，写死的灰色在浅色主题下几乎看不出来
                                style.bg(drop_tint)
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
                                    // 可点击容器必须给 id：否则 hover/active 样式不参与样式计算
                                    .id(ElementId::Name(keys.toggle.clone()))
                                    .w(px(16.0)).h(px(16.0))
                                    .flex().items_center().justify_center()
                                    .flex_shrink_0()
                                    .rounded(px(RADIUS_XS))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.hover_bg()))
                                    .active(|s| s.bg(theme.active_bg()))
                                    .on_mouse_down(MouseButton::Left, {
                                        let toggle_id = Arc::clone(&fid);
                                        cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            this.toggle_folder_expand(&toggle_id, cx);
                                        })
                                    })
                                    // 展开箭头是纯装饰的方向指示 → Muted；与文件夹树的其它图标同为密集档
                                    .child(themed_icon(
                                        if expanded { IconName::ChevronDown } else { IconName::ChevronRight },
                                        IconTier::Dense,
                                        IconTone::Muted,
                                        theme,
                                    ))
                                    .into_any_element(),
                                div()
                                    .flex_shrink_0()
                                    .child(
                                        // 文件夹 = 集合树的主语义，与 move_dialog / folder_dialog 的文件夹图标同色（Accent）
                                        themed_icon(
                                            if expanded { IconName::FolderOpen } else { IconName::FolderClosed },
                                            IconTier::Dense,
                                            IconTone::Accent,
                                            theme,
                                        ),
                                    )
                                    .into_any_element(),
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .on_mouse_move({
                                        let full_name = Arc::clone(&fname);
                                        cx.listener(move |this, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            cx.stop_propagation();
                                            this.hovered_item_name = Some(full_name.to_string());
                                            this.hovered_item_y = Some(e.position.y.into());
                                            this.hovered_item_x = Some(e.position.x.into());
                                            cx.notify();
                                        })
                                    })
                                    // 显示文案在建树时就算好了（截断只跟 name/depth 有关）
                                    .child(keys.label.clone())
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
                                        icon_button(
                                            // 行内图标按钮的 id 必须带条目 id：同层多个「更多…」共用 id 会共享 hover/active 状态
                                            ElementId::Name(keys.more.clone()),
                                            IconName::Ellipsis,
                                            theme,
                                            theme.muted_foreground,
                                            theme.foreground,
                                        )
                                            .on_mouse_down(MouseButton::Left, {
                                                let target_id = Arc::clone(&fid);
                                                cx.listener(move |this, e: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                                    cx.stop_propagation();
                                                    this.context_menu_pos = Some((e.position.x.into(), e.position.y.into()));
                                                    // 与改造前同一套「再点一次关闭」语义，只是比较时按 &str
                                                    this.context_menu_target = if this.context_menu_target.as_deref() == Some(&*target_id) {
                                                        None
                                                    } else {
                                                        Some(target_id.to_string())
                                                    };
                                                    cx.notify();
                                                })
                                            })
                                    ),
                            )
                            .child(div().into_any_element())
                            .into_any_element(),
                        if expanded && !child_items.is_empty() {
                            div().child(render_collection_panel(&child_items, cx, theme, app_state, needs_drop_refresh.clone())).into_any_element()
                        } else {
                            div().into_any_element()
                        },
                    ])
                    .into_any_element()
            }
            CollectionItem::Request { id, name, method, url, depth, keys } => {
                // 同文件夹分支：全部是引用计数克隆
                let rid = Arc::clone(id);
                let rname = Arc::clone(name);
                let rmethod = Arc::clone(method);
                let rurl = Arc::clone(url);
                let d = *depth;
                let method_clr = method_color(&rmethod);
                let req_drag = DragItem { id: Arc::clone(&rid), is_folder: false, name: Arc::clone(&rname) };

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
                    .rounded(px(RADIUS_SM))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.hover_bg()))
                    .id(ElementId::Name(keys.row.clone()))
                    .on_drag(req_drag, move |data: &DragItem, _offset, window, cx| {
                        cx.new(|_| DragPreview { label: data.name.clone().into(), bg: drag_bg, border: drag_border, fg: drag_fg })
                    })
                    .children([
                        div().w(px(d as f32 * 16.0)).flex_shrink_0().into_any_element(),
                        div().w(px(16.0)).flex_shrink_0().into_any_element(),
                        div()
                            .px_1().py_px()
                            .rounded(px(RADIUS_XS))
                            .bg(rgb(method_clr))
                            .text_xs().text_color(theme.accent_foreground)
                            .flex_shrink_0()
                            // SharedString 从 Arc<str> 构造是 O(1)（smol_str 直接接管这个 Arc），
                            // 不是复制字符串；直接传 Arc<str> 则没有 IntoElement 实现
                            .child(SharedString::from(Arc::clone(&rmethod)))
                            .into_any_element(),
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .on_mouse_move({
                                let full_name = Arc::clone(&rname);
                                cx.listener(move |this, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    cx.stop_propagation();
                                    this.hovered_item_name = Some(full_name.to_string());
                                    this.hovered_item_y = Some(e.position.y.into());
                                    this.hovered_item_x = Some(e.position.x.into());
                                    cx.notify();
                                })
                            })
                            .on_mouse_down(MouseButton::Left, {
                                let req_id = Arc::clone(&rid);
                                let req_method = Arc::clone(&rmethod);
                                let req_url = Arc::clone(&rurl);
                                let req_name = Arc::clone(&rname);
                                cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                    this.load_saved_request_by_id(&req_id, &req_method, &req_url, &req_name, _window, cx);
                                })
                            })
                            .child(keys.label.clone())
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
                                icon_button(
                                    // 行内图标按钮的 id 必须带条目 id：同层多个「更多…」共用 id 会共享 hover/active 状态
                                    ElementId::Name(keys.more.clone()),
                                    IconName::Ellipsis,
                                    theme,
                                    theme.muted_foreground,
                                    theme.foreground,
                                )
                                    .on_mouse_down(MouseButton::Left, {
                                        let target_id = Arc::clone(&rid);
                                        cx.listener(move |this, e: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                                            cx.stop_propagation();
                                            this.context_menu_pos = Some((e.position.x.into(), e.position.y.into()));
                                            this.context_menu_target = if this.context_menu_target.as_deref() == Some(&*target_id) {
                                                None
                                            } else {
                                                Some(target_id.to_string())
                                            };
                                            cx.notify();
                                        })
                                    })
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
    t: &dyn Fn(&str) -> SharedString,
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
    t: &dyn Fn(&str) -> SharedString,
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
        // 字形取 SquareTerminal（提示符方框）：本图标集没有 code 字形，
        // "命令行/代码"里它是最贴近的一档；File 已专表"请求条目"，不能再兼一个语义
        .child(menu_item(&t("context.generate_code"), IconName::SquareTerminal, theme, cx, {
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
        // 字形取 Copy：与"复制为 cURL / 复制请求体"同一个动作，必须同一个字形
        .child(menu_item(&t("context.copy_headers"), IconName::Copy, theme, cx, {
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
        // 字形取 Copy：这个动作实际是把请求序列化成 JSON 写进剪贴板（见下面的实现），
        // 对勾（Check）完全不表达"分享"；图标集里没有分享字形，ExternalLink 又被
        // "打开发布页"占用，所以按真实动作归到"复制"这一义
        .child(menu_item(&t("context.share_request"), IconName::Copy, theme, cx, {
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
        // 右键菜单项此前没有 id，`.hover()` 因此从未参与样式计算；菜单项文本互不相同，
        // 用它当 id 在同层里唯一
        .id(ElementId::from(format!("ctx-menu-{}", label)))
        .flex().flex_row().items_center().gap(px(ICON_TEXT_GAP))
        .px_3().py_1p5()
        .cursor_pointer()
        .hover(|s| s.bg(theme.hover_bg()))
        .on_mouse_down(MouseButton::Left, cx.listener(on_click))
        // 次级菜单项 → Muted；容器 hover 只换底色，图标语义色保持恒定
        .child(themed_icon(icon, IconTier::Dense, IconTone::Muted, theme))
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
        // 同 menu_item：补 id 让 hover 底色真正生效
        .id(ElementId::from(format!("ctx-menu-danger-{}", label)))
        .flex().flex_row().items_center().gap(px(ICON_TEXT_GAP))
        .px_3().py_1p5()
        .cursor_pointer()
        // 原来 hover 是实心 error 色：浅色主题下深色文字压上去对比度不够。
        // 改成 error 淡化底 + 错误色文字/图标，既明确是危险项又不牺牲可读性
        .hover(|s| s.bg(theme.error.alpha(0.16)))
        .on_mouse_down(MouseButton::Left, cx.listener(on_click))
        // 删除类菜单项 → Danger，图标与文字同色
        .child(themed_icon(icon, IconTier::Dense, IconTone::Danger, theme))
        .child(div().text_sm().text_color(theme.error).child(label.to_string()))
}
