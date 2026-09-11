use crate::ui::body::{BodyState, BodyType, FormDataParamType, RawFormat};
use crate::ui::json_editor;
use crate::ui::main_view::MainView;
use crate::ui::components::{ghost_button, segment_button, segment_group};
use crate::ui::themes::Theme;
use std::sync::Arc;
use gpui::*;
// `when` 等流式构造方法来自 prelude（与 main_view.rs 保持一致）
use gpui::prelude::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::select::Select;
use gpui_component::{Icon, IconName, Sizable, StyledExt};
use gpui_component::scroll::ScrollableElement;

pub fn render_body_panel(
    this: &mut MainView,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let body_state = this.body_state.clone();

    div()
        .flex_col()
        .flex_1()
        .gap_3()
        .p_3()
        .overflow_y_scrollbar()
        .child(render_body_type_selector(this, &body_state, cx))
        .child(render_body_content(this, body_state, window, cx))
        .into_any_element()
}

fn render_body_type_selector(
    this: &mut MainView,
    body_state: &BodyState,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let theme = this.cached_theme.clone();
    // 统一用分段按钮组，五种类型高度/圆角/字号完全一致。
    // 外层再套一层 flex_row + items_start：否则作为 flex_col 的直接子元素会被横向拉伸成整行底色。
    div().flex().flex_row().items_start().child(
        segment_group(&theme).children([
            body_type_tab(this, BodyType::None, body_state, cx),
            body_type_tab(this, BodyType::FormData, body_state, cx),
            body_type_tab(this, BodyType::UrlEncoded, body_state, cx),
            body_type_tab(this, BodyType::Raw, body_state, cx),
            body_type_tab(this, BodyType::Binary, body_state, cx),
        ]),
    )
}

fn body_type_tab(
    this: &mut MainView,
    body_type: BodyType,
    body_state: &BodyState,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let is_active = body_state.body_type == body_type;
    let theme = this.cached_theme.clone();
    let label = match body_type {
        BodyType::None => this.t("ui.none"),
        BodyType::FormData => this.t("ui.form_data"),
        BodyType::UrlEncoded => this.t("ui.url_encoded"),
        BodyType::Raw => SharedString::from("Raw"),
        BodyType::Binary => this.t("ui.binary"),
    };
    segment_button(
        format!("body-type-{:?}", body_type),
        label,
        is_active,
        &theme,
    )
    .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<MainView>| {
        this.set_body_type(body_type.to_index(), cx);
    }))
}

fn render_body_content(
    this: &mut MainView,
    // 传 Arc 而不是裸 BodyState：面板每帧都会取一份快照，Arc 只做引用计数
    body_state: Arc<BodyState>,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let theme = this.cached_theme.clone();
    if body_state.body_type == BodyType::Raw {
        render_raw_editor(this, &body_state, window, cx)
    } else if body_state.body_type == BodyType::Binary {
        // 统一成 SharedString：下面 clone 进按钮时就是引用计数 +1，不再复制文件名
        let btn_label: SharedString = match &body_state.binary_file_path {
            Some(path) => SharedString::from(
                std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone()),
            ),
            None => this.t("ui.binary_placeholder"),
        };
        let file_selected = body_state.binary_file_path.is_some();
        div()
            .flex_1()
            .flex_col()
            .pt_1()
            .child(
                crate::ui::components::ghost_button("pick-binary-file", btn_label, &theme)
                    .when(file_selected, |b| b.text_color(theme.accent))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window: &mut Window, cx: &mut Context<MainView>| {
                        this.pick_file_for_binary(window, cx);
                    })),
            )
            .into_any_element()
    } else if body_state.body_type == BodyType::FormData {
        render_key_value_editor(
            this,
            &body_state.form_data,
            "add-formdata",
            this.t("ui.add_form_data"),
            true,
            window,
            cx,
        )
    } else if body_state.body_type == BodyType::UrlEncoded {
        render_key_value_editor(
            this,
            &body_state.urlencoded_data,
            "add-urlencoded",
            this.t("ui.add_url_encoded"),
            false,
            window,
            cx,
        )
    } else {
        div().flex_1().into_any_element()
    }
}

fn render_raw_editor(
    this: &mut MainView,
    body_state: &BodyState,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let theme = this.cached_theme.clone();
    let is_json = body_state.raw_format == RawFormat::Json;
    let format_label = this.t("response.format");

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .pt_1()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .w_full()
                .gap_2()
                .child(
                    segment_group(&theme).children([
                        raw_format_btn(this, RawFormat::Json, body_state, cx).into_any_element(),
                        raw_format_btn(this, RawFormat::Xml, body_state, cx).into_any_element(),
                        raw_format_btn(this, RawFormat::Text, body_state, cx).into_any_element(),
                        raw_format_btn(this, RawFormat::Html, body_state, cx).into_any_element(),
                    ]),
                )
                .when(is_json, |el| {
                    el.child(
                        ghost_button("format-json", format_label, &theme)
                            .h(px(crate::ui::components::SEGMENT_H + 4.0))
                            .on_click(cx.listener(|this, _: &ClickEvent, window: &mut Window, cx: &mut Context<MainView>| {
                                this.format_json(window, cx);
                                cx.notify();
                            })),
                    )
                }),
        )
        .child(render_raw_editor_content(body_state, &theme, &|key| this.t(key), cx))
        .into_any_element()
}

fn render_raw_editor_content(
    body_state: &BodyState,
    theme: &Theme,
    t: &dyn Fn(&str) -> SharedString,
    cx: &mut Context<MainView>,
) -> AnyElement {
    div()
        .flex_1()
        .flex_col()
        .children([
            if body_state.raw_format == RawFormat::Json {
                Some(
                    div()
                        .flex_1()
                        .child(json_editor(
                            body_state,
                            calculate_body_line_count(body_state, cx),
                            body_state.json_error.clone(),
                            theme,
                            t,
                            cx,
                        ))
                        .into_any_element(),
                )
            } else {
                None
            },
            if body_state.raw_format != RawFormat::Json {
                let editor_input = match body_state.raw_format {
                    RawFormat::Xml => body_state.raw_content_xml.clone(),
                    RawFormat::Text => body_state.raw_content_text.clone(),
                    RawFormat::Html => body_state.raw_content_html.clone(),
                    _ => body_state.raw_content.clone(),
                };
                Some(
                    // 与 JSON 完全同一套渲染（含相同的 flex_1 + min_h(200) 高度），
                    // 之前用裸 Input：既没有边框也没有高度约束，内容根本画不出来。
                    crate::ui::json_editor::code_editor_pane(
                        &editor_input,
                        theme.background,
                        theme.foreground,
                    )
                    .into_any_element(),
                )
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>())
        .into_any_element()
}

fn raw_format_btn(
    this: &mut MainView,
    format: RawFormat,
    body_state: &BodyState,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let is_active = body_state.raw_format == format;
    let theme = this.cached_theme.clone();
    let label = match format {
        RawFormat::Json => this.t("ui.json"),
        RawFormat::Xml => this.t("ui.xml"),
        RawFormat::Text => this.t("ui.text"),
        RawFormat::Html => this.t("ui.html"),
        _ => SharedString::default(),
    };
    segment_button(
        format!("raw-fmt-{:?}", format),
        label,
        is_active,
        &theme,
    )
        .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<MainView>| {
            this.set_raw_format(format.to_index(), cx);
        }))
}

fn calculate_body_line_count(body_state: &BodyState, cx: &Context<MainView>) -> usize {
    if body_state.body_type == BodyType::Raw && body_state.raw_format == RawFormat::Json {
        let text = body_state.raw_content.read(cx).value().to_string();
        crate::ui::json_editor::count_lines(&text)
    } else {
        1
    }
}

fn render_key_value_editor(
    this: &mut MainView,
    entries: &[crate::ui::body::FormDataEntry],
    add_btn_id: &str,
    add_label: impl Into<SharedString>,
    show_type_column: bool,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let theme = this.cached_theme.clone();
    let add_btn_id = add_btn_id.to_string();
    // 列标题提前取好：表头要跟着语言切换（form-data 是 键/类型/值，urlencoded 只有 键/值）
    let t_key = this.t("ui.key");
    let t_type = this.t("ui.type");
    let t_value = this.t("ui.value");

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            div()
                .flex()
                .flex_row()
                .gap_2()
                .children({
                    let mut children: Vec<gpui::Div> = vec![
                        // 占位 24/22 与数据行的复选框、删除按钮等宽，居中的列标题才会对准输入框
                        div().w(px(24.0)).child(""),
                        div()
                            .flex_1()
                            .text_xs()
                            .text_center()
                            .text_color(theme.muted_foreground)
                            .child(t_key.clone()),
                    ];
                    if show_type_column {
                        children.push(
                            div()
                                .w(px(90.0))
                                .text_xs()
                                .text_center()
                                .text_color(theme.muted_foreground)
                                .child(t_type.clone()),
                        );
                    }
                    children.push(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_center()
                            .text_color(theme.muted_foreground)
                            .child(t_value.clone()),
                    );
                    children.push(
                        div().w(px(22.0)).child(""),
                    );
                    children
                }),
            // 表头与首行输入之间只留外层 .gap_2() 的 8px（原先叠了表头 mb_1 +
            // 本容器 py_1/mt_1 + 每行 mt_1，约 24px，标题与输入框显得隔太开）
            div()
                .flex_col()
                .gap_2()
                .children(entries.iter().enumerate().map(|(idx, entry)| {
                    let is_file =
                        entry.param_type == FormDataParamType::File;
                    let value_entity = match &entry.value {
                        crate::ui::body::FormDataValue::Text(e) => {
                            Some(e.clone())
                        }
                        crate::ui::body::FormDataValue::File(_, _) => None,
                    };
                    // 首行不加顶部外边距：表头到首行只留外层 .gap_2() 的 8px；
                    // 其余行保留 4px，输入框之间维持原来的 12px 间距
                    div()
                        .when(idx > 0, |d| d.mt_1())
                        .flex()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .children({
                            let mut children: Vec<gpui::Div> = vec![
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .text_color(if entry.enabled {
                                        theme.success
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this,
                                                  _: &MouseDownEvent,
                                                  _window: &mut Window,
                                                  cx: &mut Context<
                                                MainView,
                                            >| {
                                                this.toggle_form_data_entry(
                                                    idx, cx,
                                                );
                                            },
                                        ),
                                    )
                                    .child(if entry.enabled {
                                        "✓"
                                    } else {
                                        "○"
                                    }),
                                div().flex_1().child(
                                    Input::new(&entry.key)
                                        .small()
                                        .h(px(28.0))
                                        .bg(theme.code_background)
                                        .border_1()
                                        .border_color(theme.border)
                                        .text_color(theme.foreground),
                                ),
                            ];

                            if show_type_column {
                                children.push(
                                    div()
                                        .w(px(90.0))
                                        .child(
                                            Select::new(&entry.type_select)
                                                .small()
                                                .h(px(28.0))
                                                // 不显式给颜色时，选中文字会落到占位符的弱化色，
                                                // 看起来"不跟随主题设定的颜色"
                                                .text_color(theme.foreground),
                                        ),
                                );
                            }

                            if entry.param_type == FormDataParamType::Boolean {
                                let val_entity =
                                    entry.value.get_input_entity();
                                let is_true = val_entity
                                    .read(cx)
                                    .value()
                                    .to_string()
                                    == "true";
                                let toggle_entity = val_entity.clone();
                                children.push(
                                    div()
                                        .flex_1()
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .bg(if is_true {
                                            rgba(0x22c55e1f)
                                        } else {
                                            rgba(0xef44441f)
                                        })
                                        .text_color(if is_true {
                                            rgb(0x22c55e)
                                        } else {
                                            rgb(0xef4444)
                                        })
                                        .text_sm()
                                        .font_semibold()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                move |this,
                                                      _: &MouseDownEvent,
                                                      _window:
                                                          &mut Window,
                                                      cx:
                                                          &mut Context<
                                                          MainView,
                                                      >| {
                                                    let new_val =
                                                        if toggle_entity
                                                            .read(cx)
                                                            .value()
                                                            .to_string()
                                                            == "true"
                                                        {
                                                            "false"
                                                        } else {
                                                            "true"
                                                        };
                                                    toggle_entity.update(
                                                        cx,
                                                        move |state,
                                                              cx| {
                                                            state
                                                                .set_value(
                                                                    new_val,
                                                                    _window,
                                                                    cx,
                                                                );
                                                        },
                                                    );
                                                    cx.notify();
                                                },
                                            ),
                                        )
                                        .child(
                                            if is_true {
                                                "true"
                                            } else {
                                                "false"
                                            },
                                        ),
                                );
                            } else if !is_file {
                                children.push(div().flex_1().child(
                                    Input::new(
                                        &entry
                                            .value
                                            .get_input_entity(),
                                    )
                                    .small()
                                    .h(px(28.0))
                                    .bg(theme.code_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_color(theme.foreground),
                                ));
                            } else {
                                let file_path = entry
                                    .value
                                    .get_input_entity()
                                    .read(cx)
                                    .value()
                                    .to_string();
                                let display_path = file_path.clone();
                                let is_placeholder = display_path.is_empty();
                                children.push(
                                    div()
                                        .flex_1()
                                        .h(px(28.0))
                                        .items_center()
                                        .rounded_sm()
                                        .bg(theme.code_background)
                                        .border_1()
                                        .border_color(theme.border)
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                move |this,
                                                      _: &MouseDownEvent,
                                                      window:
                                                          &mut Window,
                                                      cx:
                                                          &mut Context<
                                                          MainView,
                                                      >| {
                                                    this.pick_file_for_form_data(
                                                        idx,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .h(px(24.0))
                                                .flex_1()
                                                .text_sm()
                                                .text_color(
                                                    if is_placeholder {
                                                        theme.muted_foreground
                                                    } else {
                                                        theme.foreground
                                                    },
                                                )
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(
                                                    if is_placeholder {
                                                        "Select file..."
                                                            .to_string()
                                                    } else {
                                                        display_path
                                                    },
                                                ),
                                        ),
                                );
                            }

                            children.push(
                                div().child(
                                div()
                                    .id(format!("{}-del-{}", add_btn_id, idx))
                                    .w(px(22.0))
                                    .h(px(22.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_color(theme.muted_foreground)
                                    .hover({
                                        let hover_bg = theme.muted_background;
                                        let err = theme.error;
                                        move |s| s.bg(hover_bg).text_color(err)
                                    })
                                    .child(Icon::new(IconName::Close).xsmall())
                                    .on_mouse_down(MouseButton::Left, cx.listener(
                                        move |this,
                                              _: &MouseDownEvent,
                                              _window: &mut Window,
                                              cx: &mut Context<MainView>| {
                                            if show_type_column {
                                                this.remove_form_data_entry(idx, _window, cx);
                                            } else {
                                                this.remove_urlencoded_entry(idx);
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                            );

                            children
                        })
                })),
            div().mt_2().child(
                crate::ui::components::icon_text_button(
                    add_btn_id.clone(),
                    IconName::Plus,
                    add_label,
                    &theme,
                    theme.accent,
                )
                    .on_click(cx.listener(
                        move |this,
                              _: &ClickEvent,
                              window: &mut Window,
                              cx: &mut Context<MainView>| {
                            if show_type_column {
                                this.add_form_data_entry(window, cx);
                            } else {
                                this.add_urlencoded_entry(window, cx);
                            }
                        },
                    )),
            ),
        ])
        .into_any_element()
}
