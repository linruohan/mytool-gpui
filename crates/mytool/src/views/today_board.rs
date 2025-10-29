use gpui::{div, App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement, ParentElement, Render, Styled, Window};

use super::Board;
use gpui_component::{dock::PanelControl, h_flex, label::Label, v_flex, ActiveTheme, IconName};
use todos::entity::ItemModel;

pub struct TodayBoard {
    focus_handle: FocusHandle,
    tasks: Vec<ItemModel>,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tasks: Vec::new(),
        }
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
impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0x33d17a).into(), gpui::rgb(0x33d17a).into()]
    }

    fn count() -> usize {
        1
    }
    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "今天需要完成的任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TodayBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodayBoard {
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
                            .child(div().text_xl().child(<TodayBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<TodayBoard as Board>::description()),
                            ),
                    ),
            )
            .child(Label::new("today"))
            .child(Label::new("today 内容"))
    }
}
