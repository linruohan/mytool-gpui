use super::{
    BoardContainer, CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard,
};
use crate::{Board, TodoStory};
use gpui::{AnyView, App, ClickEvent, Context, Entity, Hsla, Window};
use gpui_component::IconName;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BoardType {
    Inbox,     // 未完成任务
    Today,     // 今日任务
    Scheduled, // 计划任务
    Pinboard,  // 关注任务
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

    pub fn container(&self, window: &mut Window, cx: &mut App) -> Entity<BoardContainer> {
        match self {
            Self::Inbox => BoardContainer::panel::<InboxBoard>(window, cx),
            Self::Today => BoardContainer::panel::<TodayBoard>(window, cx),
            Self::Scheduled => BoardContainer::panel::<ScheduledBoard>(window, cx),
            Self::Pinboard => BoardContainer::panel::<PinBoard>(window, cx),
            Self::Labels => BoardContainer::panel::<LabelsBoard>(window, cx),
            Self::Completed => BoardContainer::panel::<CompletedBoard>(window, cx),
        }
    }
    pub fn handler1(
        &self,
    ) -> impl Fn(&mut TodoStory, &ClickEvent, &mut Window, &mut Context<TodoStory>) + 'static {
        let _item = *self;
        move |_this, _, _window, _cx| {
            // this.is_board_active = true;
            // if this.active_boards.contains_key(&item) {
            //     this.active_boards.remove(&item);
            // } else {
            //     this.active_boards.insert(item.label(), true);
            //     // this.active_boards.remove(&this.active_board.unwrap()); // 我自己写的不一定正确
            // }
            //
            // this.active_board = Some(self.container(window, cx).into());
            // cx.notify();
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
    pub fn colors(&self) -> Vec<Hsla> {
        match self {
            Self::Inbox => InboxBoard::colors(),
            Self::Today => TodayBoard::colors(),
            Self::Scheduled => ScheduledBoard::colors(),
            Self::Pinboard => PinBoard::colors(),
            Self::Labels => LabelsBoard::colors(),
            Self::Completed => CompletedBoard::colors(),
        }
    }
}
