use crate::ui::Theme;
use gpui::*;

pub fn render_collection_panel(t: impl Fn(&str) -> String, theme: &Theme) -> impl IntoElement {
    div()
        .id("sidebar-collections")
        .p_2()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(t("sidebar.collections"))
}
