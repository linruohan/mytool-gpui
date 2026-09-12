use gpui::{Hsla, ParentElement, SharedString};
use gpui_component::{Colorize as _, Sizable, tag::Tag};

/// Label chip backed by `gpui_component::tag::Tag`.
pub fn label_chip(name: impl Into<SharedString>, color_hex: &str) -> Tag {
    let color = Hsla::parse_hex(color_hex).unwrap_or_default();
    Tag::custom(color.lighten(0.35), color, color.darken(0.1))
        .small()
        .rounded_full()
        .child(name.into())
}
