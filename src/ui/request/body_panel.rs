use crate::ui::body::{BodyState, BodyType, FormDataParamType, RawFormat};
use crate::ui::json_editor;
use crate::ui::main_view::MainView;
use crate::ui::themes::Theme;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::{IconName, Sizable, StyledExt};

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
        .overflow_y_hidden()
        .child(render_body_type_selector(this, &body_state, cx))
        .child(render_body_content(this, body_state, window, cx))
        .into_any_element()
}

fn render_body_type_selector(
    this: &mut MainView,
    body_state: &BodyState,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_4()
        .children([
            body_type_tab(this, BodyType::None, body_state, cx),
            body_type_tab(this, BodyType::FormData, body_state, cx),
            body_type_tab(this, BodyType::UrlEncoded, body_state, cx),
            body_type_tab(this, BodyType::Raw, body_state, cx),
            body_type_tab(this, BodyType::Binary, body_state, cx),
        ])
}

fn body_type_tab(
    this: &mut MainView,
    body_type: BodyType,
    body_state: &BodyState,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let is_active = body_state.body_type == body_type;
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let label = match body_type {
        BodyType::None => this.t("ui.none"),
        BodyType::FormData => this.t("ui.form_data"),
        BodyType::UrlEncoded => this.t("ui.url_encoded"),
        BodyType::Raw => "Raw".to_string(),
        BodyType::Binary => this.t("ui.binary"),
    };
    let min_w = match body_type {
        BodyType::UrlEncoded => px(130.0),
        _ => px(70.0),
    };
    div()
        .text_sm()
        .cursor_pointer()
        .min_w(min_w)
        .px_2()
        .py_1()
        .rounded_sm()
        .bg(if is_active {
            theme.muted_background
        } else {
            theme.input_background
        })
        .text_color(if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.set_body_type(body_type.to_index(), cx);
            }),
        )
        .child(label)
}

fn render_body_content(
    this: &mut MainView,
    body_state: BodyState,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    if body_state.body_type == BodyType::Raw {
        render_raw_editor(this, &body_state, window, cx)
    } else if body_state.body_type == BodyType::Binary {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.code_background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child("Binary content not supported yet")
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
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .w_full()
                .gap_2()
                .px_1()
                .py_1()
                .bg(theme.muted_background)
                .children([
                    raw_format_btn(this, RawFormat::Json, body_state, cx).into_any_element(),
                    raw_format_btn(this, RawFormat::Xml, body_state, cx).into_any_element(),
                    raw_format_btn(this, RawFormat::Text, body_state, cx).into_any_element(),
                    raw_format_btn(this, RawFormat::Html, body_state, cx).into_any_element(),
                ]),
        )
        .child(render_raw_editor_content(body_state, &theme, cx))
        .into_any_element()
}

fn render_raw_editor_content(
    body_state: &BodyState,
    theme: &Theme,
    cx: &mut Context<MainView>,
) -> AnyElement {
    div()
        .flex_1()
        .flex_col()
        .overflow_hidden()
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
                    div()
                        .flex_1()
                        .bg(theme.code_background)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .overflow_hidden()
                        .child(
                            Input::new(&editor_input)
                                .h(px(body_state.raw_editor_height))
                                .w_full()
                                .bg(theme.background)
                                .bordered(true),
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
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let label = match format {
        RawFormat::Json => this.t("ui.json"),
        RawFormat::Xml => this.t("ui.xml"),
        RawFormat::Text => this.t("ui.text"),
        RawFormat::Html => this.t("ui.html"),
        _ => "".to_string(),
    };
    div()
        .text_sm()
        .cursor_pointer()
        .min_w(px(50.0))
        .px_2()
        .py_px()
        .rounded_sm()
        .bg(if is_active {
            theme.muted_background
        } else {
            theme.input_background
        })
        .text_color(if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<MainView>| {
                this.set_raw_format(format.to_index(), cx);
            }),
        )
        .child(label)
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
    add_label: String,
    show_type_column: bool,
    window: &mut Window,
    cx: &mut Context<MainView>,
) -> AnyElement {
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
    let entries: Vec<crate::ui::body::FormDataEntry> = entries.to_vec();
    let add_btn_id = add_btn_id.to_string();

    div()
        .flex_col()
        .flex_1()
        .gap_2()
        .children([
            div()
                .flex()
                .flex_row()
                .gap_2()
                .mb_1()
                .children({
                    let mut children: Vec<gpui::Div> = vec![
                        div()
                            .w(px(30.0))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(""),
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Key"),
                    ];
                    if show_type_column {
                        children.push(
                            div()
                                .w(px(90.0))
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Type"),
                        );
                    }
                    children.push(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Value"),
                    );
                    children.push(
                        div()
                            .w(px(30.0))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(""),
                    );
                    children
                }),
            div()
                .flex_col()
                .gap_2()
                .py_1()
                .mt_1()
                .children(entries.iter().enumerate().map(|(idx, entry)| {
                    let is_file =
                        entry.param_type == FormDataParamType::File;
                    let value_entity = match &entry.value {
                        crate::ui::body::FormDataValue::Text(e) => {
                            Some(e.clone())
                        }
                        crate::ui::body::FormDataValue::File(_, _) => None,
                    };
                    let param_type_copy = entry.param_type;

                    div()
                        .mt_1()
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
                                        .h(px(32.0))
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
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .rounded_sm()
                                        .bg(theme.code_background)
                                        .border_1()
                                        .border_color(theme.border)
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
                                                    let next_type =
                                                        match param_type_copy
                                                        {
                                                            FormDataParamType::Text => FormDataParamType::Boolean,
                                                            FormDataParamType::Boolean => FormDataParamType::Number,
                                                            FormDataParamType::Number => FormDataParamType::File,
                                                            FormDataParamType::File => FormDataParamType::Array,
                                                            FormDataParamType::Array => FormDataParamType::Text,
                                                        };
                                                    this.set_form_data_param_type(
                                                        idx,
                                                        next_type,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ),
                                        )
                                        .children([
                                            div()
                                                .text_sm()
                                                .text_color(rgb(
                                                    0xe0e0e0,
                                                ))
                                                .child(
                                                    match entry.param_type {
                                                        FormDataParamType::Text => "Text",
                                                        FormDataParamType::Boolean => "Boolean",
                                                        FormDataParamType::Number => "Number",
                                                        FormDataParamType::File => "File",
                                                        FormDataParamType::Array => "Array",
                                                    },
                                                ),
                                            div()
                                                .h(px(24.0))
                                                .text_sm()
                                                .text_color(rgb(
                                                    0x888888,
                                                ))
                                                .child("▼"),
                                        ]),
                                );
                            }

                            if !is_file {
                                children.push(div().flex_1().child(
                                    Input::new(
                                        &entry
                                            .value
                                            .get_input_entity(),
                                    )
                                    .small()
                                    .h(px(32.0))
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
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Button::new(idx.to_string())
                                            .small()
                                            .icon(IconName::Close)
                                            .text_color(theme.muted_foreground)
                                            .bg(theme.background)
                                            .on_click(cx.listener(
                                                move |this,
                                                      _: &ClickEvent,
                                                      _window:
                                                          &mut Window,
                                                      cx:
                                                          &mut Context<
                                                          MainView,
                                                      >| {
                                                    if show_type_column {
                                                        this.remove_form_data_entry(idx);
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
            div().mt_2().px_1().py_1().child(
                Button::new(add_btn_id.clone())
                    .min_w(px(120.0))
                    .px_2()
                    .py_1()
                    .text_sm()
                    .icon(IconName::Plus)
                    .text_color(theme.accent)
                    .bg(theme.background)
                    .label(add_label)
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
