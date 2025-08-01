use super::{
    CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard, TodoContainer,
};
use crate::{Mytool, TodoStory};
use gpui::{AnyView, App, ClickEvent, Context, Entity, Focusable, Hsla, Render, Window};
use gpui_component::IconName;

pub trait Board: Mytool + Render + Focusable + Sized {
    fn icon() -> IconName;
    fn color() -> Hsla;
    fn count() -> usize;
}
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BoardType {
    Inbox,     // 未完成任务
    Today,     // 今日任务
    Scheduled, // 计划任务
    Pinboard,  // 挂起任务
    Labels,    // 标签list
    Completed, // 已完成任务
}

impl BoardType {
    pub fn view(&self, window: &mut Window, cx: &mut App) -> AnyView {
        match self {
            Self::Inbox => InboxBoard::view(window, cx).into(),
            Self::Today => TodayBoard::view(window, cx).into(),
            Self::Scheduled => ScheduledBoard::view(window, cx).into(),
            Self::Pinboard => PinBoard::view(window, cx).into(),
            Self::Labels => LabelsBoard::view(window, cx).into(),
            Self::Completed => CompletedBoard::view(window, cx).into(),
        }
    }

    pub fn container(&self, window: &mut Window, cx: &mut App) -> Entity<TodoContainer> {
        match self {
            Self::Inbox => TodoContainer::panel::<InboxBoard>(window, cx),
            Self::Today => TodoContainer::panel::<TodayBoard>(window, cx),
            Self::Scheduled => TodoContainer::panel::<ScheduledBoard>(window, cx),
            Self::Pinboard => TodoContainer::panel::<PinBoard>(window, cx),
            Self::Labels => TodoContainer::panel::<LabelsBoard>(window, cx),
            Self::Completed => TodoContainer::panel::<CompletedBoard>(window, cx),
        }
    }
    pub fn handler(
        &self,
    ) -> impl Fn(&mut TodoStory, &ClickEvent, &mut Window, &mut Context<TodoStory>) + 'static {
        let item = *self;
        move |this, _, _, cx| {
            println!("laste_board:{:?}", this.active_board);
            println!("Clicked on item: {}", item.label(),);
            this.is_board_active = true;
            if this.active_boards.contains_key(&item) {
                this.active_boards.remove(&item);
            } else {
                this.active_boards.insert(item, true);
                this.active_boards.remove(&this.active_board.unwrap()); // 我自己写的不一定正确
            }

            this.active_board = Some(item);
            cx.notify();
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Today => "Today",
            Self::Scheduled => "Scheduled",
            Self::Pinboard => "Pinboard",
            Self::Labels => "Labels",
            Self::Completed => "Completed",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Inbox => InboxBoard::icon(),
            Self::Today => TodayBoard::icon(),
            Self::Scheduled => ScheduledBoard::icon(),
            Self::Pinboard => PinBoard::icon(),
            Self::Labels => LabelsBoard::icon(),
            Self::Completed => CompletedBoard::icon(),
        }
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Inbox => InboxBoard::count(),
            Self::Today => TodayBoard::count(),
            Self::Scheduled => ScheduledBoard::count(),
            Self::Pinboard => PinBoard::count(),
            Self::Labels => LabelsBoard::count(),
            Self::Completed => CompletedBoard::count(),
        }
    }
    pub fn color(&self) -> Hsla {
        match self {
            Self::Inbox => InboxBoard::color(),
            Self::Today => TodayBoard::color(),
            Self::Scheduled => ScheduledBoard::color(),
            Self::Pinboard => PinBoard::color(),
            Self::Labels => LabelsBoard::color(),
            Self::Completed => CompletedBoard::color(),
        }
    }
}
