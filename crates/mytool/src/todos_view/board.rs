use super::{
    CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard, TodoContainer,
};
use crate::Mytool;
use gpui::{AnyView, App, Entity, Focusable, Hsla, Render, Window};
use gpui_component::IconName;

pub trait Board: Mytool + Render + Focusable + Sized {
    fn icon() -> IconName;
    fn color() -> Hsla;
    fn count() -> usize;
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
}
