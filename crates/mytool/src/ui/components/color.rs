use gpui::{Entity, Hsla};
use gpui_component::color_picker::{ColorPicker, ColorPickerState};
use itertools::Itertools as _;
use todos::utils::Util;

/// Palette used by project / label color pickers.
pub fn featured_todo_colors() -> Vec<Hsla> {
    let util = Util::default();
    util.get_colors()
        .keys()
        .sorted()
        .map(|k| Hsla::from(gpui::rgb(util.get_color_u32_by_key(k.to_string()))))
        .collect()
}

/// Color picker preloaded with the todo color palette.
pub fn todo_color_picker(state: &Entity<ColorPickerState>) -> ColorPicker {
    ColorPicker::new(state).featured_colors(featured_todo_colors())
}
