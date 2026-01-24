mod database;
mod item;
mod item_completed;
mod item_inbox;
mod item_pinned;
mod item_scheduled;
mod item_today;
mod label;
mod project;
mod section;

pub use database::*;
use gpui::App;
pub use item::*;
pub use item_completed::*;
pub use item_inbox::*;
pub use item_pinned::*;
pub use item_scheduled::*;
pub use item_today::*;
pub use label::*;
pub use project::*;
pub use section::*;
pub fn state_init(cx: &mut App) {
    // item
    ItemState::init(cx);
    // other item
    InboxItemState::init(cx);
    TodayItemState::init(cx);
    ScheduledItemState::init(cx);
    PinnedItemState::init(cx);
    CompleteItemState::init(cx);
    // project
    ProjectState::init(cx);
    // label
    LabelState::init(cx);
    // section
    SectionState::init(cx);
}
