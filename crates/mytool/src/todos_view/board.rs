use super::{
    CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard, TodoContainer,
};
use crate::{Mytool, ProjectItem};
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
    Project,
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
            Self::Project => ProjectItem::view(window, cx).into(),
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
            Self::Project => TodoContainer::panel::<ProjectItem>(window, cx),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Today => "Today",
            Self::Scheduled => "Scheduled",
            Self::Pinboard => "Pinboard",
            Self::Labels => "Labels",
            Self::Completed => "Completed",
            Self::Project => "Project",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Inbox => IconName::MailboxSymbolic,
            Self::Today => IconName::StarOutlineThickSymbolic,
            Self::Scheduled => IconName::MonthSymbolic,
            Self::Pinboard => IconName::PinSymbolic,
            Self::Labels => IconName::TagOutlineSymbolic,
            Self::Completed => IconName::CheckRoundOutlineSymbolic,
            Self::Project => IconName::ProcessErrorSymbolic,
        }
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Inbox => 10,
            Self::Today => 2,
            Self::Scheduled => 3,
            Self::Pinboard => 5,
            Self::Labels => 6,
            Self::Completed => 2,
            Self::Project => 10,
        }
    }
    pub fn color(&self) -> Hsla {
        match self {
            Self::Inbox => gpui::rgb(0x99c1f1).into(),
            Self::Today => gpui::rgb(0x33d17a).into(),
            Self::Scheduled => gpui::rgb(0xdc8add).into(),
            Self::Pinboard => gpui::rgb(0xf66151).into(),
            Self::Labels => gpui::rgb(0xcdab8f).into(),
            Self::Completed => gpui::rgb(0xffbe6f).into(),
            Self::Project => gpui::rgb(0x33D17A).into(),
        }
    }
}
