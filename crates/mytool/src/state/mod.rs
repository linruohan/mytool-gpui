mod database;
mod item;
mod label;
mod project;

pub use database::*;
use gpui::App;
pub use item::*;
pub use label::*;
pub use project::*;

pub fn state_init(cx: &mut App) {
    ItemState::init(cx);
    LabelState::init(cx);
    ProjectState::init(cx);
}
