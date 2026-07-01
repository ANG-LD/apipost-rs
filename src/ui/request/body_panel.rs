use crate::ui::body::{BodyState, BodyType, FormDataParamType, RawFormat};
use crate::ui::json_editor;
use crate::ui::main_view::MainView;
use crate::ui::themes::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::select::Select;
use gpui_component::{IconName, Sizable, StyledExt};
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
    Button::new(format!("body-type-{:?}", body_type))
        .label(label)
        .xsmall()
        .when(is_active, |b| {
            b.bg(theme.accent).text_color(theme.accent_foreground)
        })
        .when(!is_active, |b| {
            b.bg(theme.input_background).text_color(theme.muted_foreground)
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<MainView>| {
            this.set_body_type(body_type.to_index(), cx);
        }))
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
        let btn_label = match &body_state.binary_file_path {
            Some(path) => {
                std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            }
            None => this.t("ui.binary_placeholder"),
        };
        let file_selected = body_state.binary_file_path.is_some();
        div()
            .flex_1()
            .flex_col()
            .pt_1()
            .child(
                Button::new("pick-binary-file")
                    .label(btn_label)
                    .small()
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
    let theme = Theme::from_str(&this.app_state.lock().unwrap().theme_name);
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
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .children([
                            raw_format_btn(this, RawFormat::Json, body_state, cx).into_any_element(),
                            raw_format_btn(this, RawFormat::Xml, body_state, cx).into_any_element(),
                            raw_format_btn(this, RawFormat::Text, body_state, cx).into_any_element(),
                            raw_format_btn(this, RawFormat::Html, body_state, cx).into_any_element(),
                        ]),
                )
                .when(is_json, |el| {
                    el.child(
                        Button::new("format-json")
                            .label(format_label)
                            .xsmall()
                            .rounded_sm()
                            .bg(theme.input_background)
                            .text_color(theme.foreground)
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
    t: &dyn Fn(&str) -> String,
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
                    div()
                        .flex_1()
                        .child(
                            Input::new(&editor_input)
                                .h_full()
                                .w_full()
                                .bg(theme.background),
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
    Button::new(format!("raw-fmt-{:?}", format))
        .label(label)
        .xsmall()
        .when(is_active, |b| {
            b.bg(theme.accent).text_color(theme.accent_foreground)
        })
        .when(!is_active, |b| {
            b.bg(theme.input_background).text_color(theme.muted_foreground)
        })
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
                                                .h(px(28.0)),
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
