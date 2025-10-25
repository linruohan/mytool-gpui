use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window,
};

use super::Board;
use crate::Mytool;
use gpui_component::{IconName, Theme, dock::PanelControl, label::Label, v_flex};
use todos::entity::ItemModel;

pub struct TodayBoard {
    focus_handle: FocusHandle,
    is_dark: bool,
    tasks: Vec<ItemModel>,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let theme_mode = Theme::global(cx).mode;
        Self {
            focus_handle: cx.focus_handle(),
            is_dark: theme_mode.is_dark(),
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
    fn icon(&self) -> IconName {
        IconName::StarOutlineThickSymbolic
    }
    fn color(&self) -> Hsla {
        gpui::rgb(0x33d17a).into()
    }

    fn count(&self) -> usize {
        self.tasks.len()
    }
}
impl Mytool for TodayBoard {
    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "."
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
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(Label::new("today"))
            .child(Label::new("today 内容"))
    }
}
