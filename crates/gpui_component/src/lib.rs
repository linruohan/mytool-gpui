mod colors;
mod event;
mod icon;
pub mod scroll;
mod styled;
pub mod theme;
mod title_bar;

pub use colors::*;
pub use event::InteractiveElementExt;
pub use icon::*;
pub use styled::*;
pub use theme::*;
pub use title_bar::*;

use gpui::App;
use std::ops::Deref;

rust_i18n::i18n!("locales", fallback = "zh-CN");

pub fn init(cx: &mut App) {
    theme::init(cx);
}

#[inline]
pub fn locale() -> impl Deref<Target = str> {
    rust_i18n::locale()
}

#[inline]
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale)
}

#[inline]
pub(crate) fn measure_enable() -> bool {
    std::env::var("ZED_MEASUREMENTS").is_ok()
}
