use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, Theme, dock::PanelControl, h_flex, label::Label, v_flex,
};
use todos::entity::ItemModel;

use crate::Board;

pub struct ScheduledBoard {
    focus_handle: FocusHandle,
    tasks: Vec<ItemModel>,
}

impl ScheduledBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let _theme_mode = Theme::global(cx).mode;
        Self { focus_handle: cx.focus_handle(), tasks: Vec::new() }
    }

    pub fn tasks(&self) -> &[ItemModel] {
        &self.tasks
    }

    pub fn add_task(&mut self, task: ItemModel) {
        self.tasks.push(task);
    }

    pub fn clear_tasks(&mut self) {
        self.tasks.clear();
    }
}
impl Board for ScheduledBoard {
    fn icon() -> IconName {
        IconName::MonthSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xdc8add).into(), gpui::rgb(0x9141ac).into()]
    }

    fn count() -> usize {
        1
    }

    fn title() -> &'static str {
        "Scheduled"
    }

    fn description() -> &'static str {
        "计划中任务，在其他时间去执行的任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScheduledBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScheduledBoard {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .overflow_x_hidden()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xl().child(<ScheduledBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<ScheduledBoard as Board>::description()),
                            ),
                    ),
            )
            .child(Label::new("scheduled"))
            .child(Label::new("scheduled 内容"))
    }
}
