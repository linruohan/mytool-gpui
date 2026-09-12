use gpui::{Hsla, IntoElement, ParentElement, SharedString, Styled, div, px};
use gpui_component::{Colorize as _, Sizable, tag::Tag};

/// Parse a label hex color, falling back to a muted blue.
pub fn label_color(color_hex: &str) -> Hsla {
    Hsla::parse_hex(color_hex).unwrap_or(Hsla { h: 0.6, s: 0.45, l: 0.5, a: 1.0 })
}

/// Small color swatch used in pickers.
pub fn label_color_dot(color_hex: &str) -> impl IntoElement {
    div().size(px(10.)).rounded_full().flex_shrink_0().bg(label_color(color_hex))
}

/// Label chip backed by `gpui_component::tag::Tag`.
///
/// Always use a pastel fill with dark ink so neon yellows/cyans stay readable.
pub fn label_chip(name: impl Into<SharedString>, color_hex: &str) -> Tag {
    let color = label_color(color_hex);
    let bg = Hsla { h: color.h, s: color.s.clamp(0.28, 0.62), l: 0.86, a: 1.0 };
    let fg = Hsla { h: color.h, s: (color.s * 0.5).clamp(0.28, 0.52), l: 0.26, a: 1.0 };
    Tag::custom(bg, fg, color.opacity(0.4)).small().rounded_full().child(name.into())
}
