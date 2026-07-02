use std::sync::Arc;

use crate::app::database::HistoryEntry;
use crate::app::HttpResponse;
use crate::ui::body::{BodyType, RawFormat};
use crate::ui::components::method_color;
use crate::ui::headers::HeaderEntry;
use crate::ui::main_view::{MainView, ParamEntry};
use crate::ui::Theme;
use gpui::*;
use gpui_component::input::InputState;
use gpui_component::IndexPath;

pub fn render_history_panel(
    history: &[HistoryEntry],
    t: impl Fn(&str) -> String,
    cx: &mut Context<MainView>,
    theme: &Theme,
) -> impl IntoElement {
    if history.is_empty() {
        div()
            .id("history-empty")
            .p_4()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(t("ui.no_history"))
    } else {
        div()
            .id("history-list")
            .flex_col()
            .gap_1()
            .overflow_y_scroll()
            .p_2()
            .children(history.iter().map(|entry| {
                let method_clr = method_color(&entry.method);
                let entry_url = entry.url.clone();
                let entry_method = entry.method.clone();
                let entry_clone = entry.clone();
                let entry_response_body = entry.response_body.clone();
                let entry_response_headers = entry.response_headers.clone();
                let entry_response_time_ms = entry.response_time_ms;
                let entry_response_size =
                    entry.response_body.as_ref().map(|b| b.len() as i64);
                let display_method = entry.method.clone();
                let display_url = entry.url.clone();

                div()
                    .w_full()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.code_background))
                    .bg(theme.muted_background)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(
                            move |this,
                                  _: &MouseDownEvent,
                                  _window: &mut Window,
                                  cx: &mut Context<MainView>| {
                                this.url = entry_url.clone();
                                this.is_importing_curl = true;
                                let url_str = entry_url.clone();
                                this.url_input.update(cx, |state, cx| {
                                    state.set_value(&url_str, _window, cx);
                                });
                                this.method = entry_method.clone();
                                let method_upper = entry_method.to_uppercase();
                                let method_idx = [
                                    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD",
                                    "OPTIONS",
                                ]
                                .iter()
                                .position(|&m| m == method_upper)
                                .unwrap_or(0);
                                let idx_path = Some(IndexPath::new(method_idx));
                                this.method_select.update(cx, |state, cx| {
                                    state.set_selected_index(idx_path, _window, cx);
                                });

                                if let Some(ref body_content) = entry_clone.body {
                                    // 从 Content-Type header 检测格式
                                    let content_type = entry_clone.headers.as_ref().and_then(|h| {
                                        h.lines().find_map(|line| {
                                            let (k, v) = line.split_once(':')?;
                                            if k.trim().eq_ignore_ascii_case("content-type") {
                                                Some(v.trim().to_string())
                                            } else {
                                                None
                                            }
                                        })
                                    });
                                    let detected_format =
                                        RawFormat::detect(content_type.as_deref(), body_content);
                                    let formatted_body =
                                        RawFormat::Json.format_body(body_content);
                                    // 设置 body 类型和格式
                                    this.body_state.body_type = BodyType::Raw;
                                    this.body_state.raw_format = detected_format;
                                    let rf_idx = detected_format.to_index();
                                    this.raw_format_select.update(cx, |state, cx| {
                                        state.set_selected_index(Some(IndexPath::new(rf_idx)), _window, cx);
                                    });
                                    let bt_idx = BodyType::Raw.to_index();
                                    this.body_type_select.update(cx, |state, cx| {
                                        state.set_selected_index(Some(gpui_component::IndexPath::new(bt_idx)), _window, cx);
                                    });
                                    this.body_state.raw_content.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                &formatted_body,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.body_state.raw_content_xml.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                body_content,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.body_state.raw_content_text.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                body_content,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.body_state.raw_content_html.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                body_content,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                } else {
                                    // 无 body 时重置为 None
                                    this.body_state.body_type = BodyType::None;
                                    this.body_state.raw_content.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.body_state.raw_content_xml.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.body_state.raw_content_text.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.body_state.raw_content_html.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    let bt_idx = BodyType::None.to_index();
                                    this.body_type_select.update(cx, |state, cx| {
                                        state.set_selected_index(Some(IndexPath::new(bt_idx)), _window, cx);
                                    });
                                }

                                if let Some(ref headers_text) = entry_clone.headers {
                                    this.headers.clear();
                                    for line in headers_text.lines() {
                                        if let Some(colon_pos) = line.find(':') {
                                            let key = line[..colon_pos]
                                                .trim()
                                                .to_string();
                                            let value = line[colon_pos + 1..]
                                                .trim()
                                                .to_string();
                                            if !key.is_empty() {
                                                this.headers.push(
                                                    HeaderEntry::new(
                                                        _window, cx,
                                                    ),
                                                );
                                                let len = this.headers.len();
                                                let header =
                                                    &mut this.headers[len - 1];
                                                header.key.update(
                                                    cx,
                                                    |state, cx| {
                                                        state.set_value(
                                                            &key,
                                                            _window,
                                                            cx,
                                                        );
                                                    },
                                                );
                                                header.value.update(
                                                    cx,
                                                    |state, cx| {
                                                        state.set_value(
                                                            &value,
                                                            _window,
                                                            cx,
                                                        );
                                                    },
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    this.headers.clear();
                                }

                                this.params.clear();
                                if let Some(query_start) = entry_url.find('?') {
                                    let query_string =
                                        &entry_url[query_start + 1..];
                                    for param in query_string.split('&') {
                                        if let Some(eq_pos) = param.find('=') {
                                            let key =
                                                urlencoding::decode(
                                                    &param[..eq_pos],
                                                )
                                                .map(|s| s.to_string())
                                                .unwrap_or_else(|_| {
                                                    param[..eq_pos].to_string()
                                                });
                                            let value =
                                                urlencoding::decode(
                                                    &param[eq_pos + 1..],
                                                )
                                                .map(|s| s.to_string())
                                                .unwrap_or_else(|_| {
                                                    param[eq_pos + 1..]
                                                        .to_string()
                                                });
                                            let key_entity = cx.new(|cx| {
                                                InputState::new(_window, cx)
                                                    .default_value(&key)
                                            });
                                            let value_entity = cx.new(|cx| {
                                                InputState::new(_window, cx)
                                                    .default_value(&value)
                                            });
                                            this.params.push(ParamEntry {
                                                key: key_entity,
                                                value: value_entity,
                                                enabled: true,
                                            });
                                        }
                                    }
                                }
                                this.rebuild_param_subscriptions(_window, cx);

                                if let Some(status) = entry_clone.response_status {
                                    let resp_body = entry_response_body
                                        .clone()
                                        .unwrap_or_default();
                                    let resp_headers: std::collections::HashMap<
                                        String,
                                        String,
                                    > = entry_response_headers
                                        .as_ref()
                                        .and_then(|h| serde_json::from_str(h).ok())
                                        .unwrap_or_default();
                                    let content_type = resp_headers
                                        .get("content-type")
                                        .cloned();
                                    let response = HttpResponse {
                                        status: status as u16,
                                        headers: resp_headers,
                                        body: Arc::from(resp_body.clone()),
                                        raw_body: None,
                                        time_ms: entry_response_time_ms
                                            .unwrap_or(0),
                                        size_bytes: entry_response_size
                                            .unwrap_or(0),
                                        cookies: Vec::new(),
                                    };
                                    this.response = Some(response);
                                    this.response_raw_format = RawFormat::detect(
                                        content_type.as_deref(),
                                        &resp_body,
                                    );
                                    let json_body =
                                        RawFormat::Json.format_body(&resp_body);
                                    this.response_input.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                &json_body,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.response_xml_input.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                &resp_body,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.response_text_input.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                &resp_body,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.response_html_input.update(
                                        cx,
                                        |state, cx| {
                                            state.set_value(
                                                &resp_body,
                                                _window,
                                                cx,
                                            );
                                        },
                                    );
                                    this.update_pretty_editor(_window, cx);
                                } else {
                                    this.response = None;
                                    this.response_input.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.response_xml_input.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.response_text_input.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                    this.response_html_input.update(cx, |state, cx| {
                                        state.set_value("", _window, cx);
                                    });
                                }
                                this.rebuild_header_subscriptions(_window, cx);
                                this.auto_detect_body_type_from_headers(_window, cx);
                                this.last_synced_url = entry_url.clone();
                                this.is_importing_curl = false;

                                cx.notify();
                            },
                        ),
                    )
                    .children([
                        div().flex().items_center().gap_2().children([
                            div()
                                .px_1()
                                .py_px()
                                .rounded_sm()
                                .bg(rgb(method_clr))
                                .text_xs()
                                .text_color(rgb(0xffffff))
                                .child(display_method),
                            div()
                                .flex_1()
                                .text_ellipsis()
                                .text_xs()
                                .text_color(theme.foreground)
                                .child(display_url),
                        ]),
                        if let Some(status) = entry.response_status {
                            let status_color =
                                if (200..300).contains(&status) {
                                    0x22c55e
                                } else {
                                    0xef4444
                                };
                            div()
                                .text_xs()
                                .text_color(rgb(status_color))
                                .child(format!(
                                    "{} ({})",
                                    status,
                                    entry
                                        .response_time_ms
                                        .map(|t| format!("{}ms", t))
                                        .unwrap_or_default()
                                ))
                        } else {
                            div()
                        },
                    ])
            }))
    }
}
