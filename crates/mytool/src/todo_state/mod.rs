mod complete_item;
mod database;
mod item;
mod label;
mod pinned_item;
mod project;
mod project_item;
mod scheduled_item;
mod section;
mod today_item;

pub use complete_item::*;
pub use database::*;
use gpui::App;
pub use item::*;
pub use label::*;
pub use pinned_item::*;
pub use project::*;
pub use project_item::*;
pub use scheduled_item::*;
pub use section::*;
pub use today_item::*;
pub fn state_init(cx: &mut App) {
    // item
    ItemState::init(cx);
    SectionState::init(cx);
    // other item
    TodayItemState::init(cx);
    ScheduledItemState::init(cx);
    PinnedItemState::init(cx);
    CompleteItemState::init(cx);
    // project
    ProjectState::init(cx);
    // label
    LabelState::init(cx);
}
