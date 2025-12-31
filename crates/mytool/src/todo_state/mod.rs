mod database;
mod item;
mod item_complete;
mod item_pinned;
mod item_project;
mod item_scheduled;
mod item_today;
mod label;
mod project;
mod section;

pub use database::*;
use gpui::App;
pub use item::*;
pub use item_complete::*;
pub use item_pinned::*;
pub use item_project::*;
pub use item_scheduled::*;
pub use item_today::*;
pub use label::*;
pub use project::*;
pub use section::*;
pub fn state_init(cx: &mut App) {
    // item
    ItemState::init(cx);
    // other item
    TodayItemState::init(cx);
    ScheduledItemState::init(cx);
    PinnedItemState::init(cx);
    CompleteItemState::init(cx);
    ProjectItemState::init(cx);
    // project
    ProjectState::init(cx);
    SectionState::init(cx);
    // label
    LabelState::init(cx);
}
