pub mod icon;
mod time;

pub mod sidebar;

pub use icon::*;
pub use time::*;
rust_i18n::i18n!("locales", fallback = "en");
