use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window,
};

use super::Board;
use crate::Mytool;
use gpui_component::{IconName, Theme, dock::PanelControl, label::Label, v_flex};
use todos::entity::ItemModel;

pub struct PinBoard {
    focus_handle: FocusHandle,
    is_dark: bool,
    tasks: Vec<ItemModel>,
}

impl PinBoard {
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
impl Board for PinBoard {
    fn icon(&self) -> IconName {
        IconName::PinSymbolic
    }
    fn color(&self) -> Hsla {
        let hex = if self.is_dark { 0xf66151 } else { 0xed333b };
        gpui::rgb(hex).into()
    }
    fn count(&self) -> usize {
        self.tasks.len()
    }
}
impl Mytool for PinBoard {
    fn title() -> &'static str {
        "Pinboard"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for PinBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PinBoard {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(Label::new("pinned"))
            .child(Label::new("pinned 内容"))
    }
}
