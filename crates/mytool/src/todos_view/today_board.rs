use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window,
};

use gpui_component::{IconName, dock::PanelControl, label::Label, v_flex};

use super::Board;
use crate::Mytool;

pub struct TodayBoard {
    focus_handle: FocusHandle,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}
impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn color() -> Hsla {
        gpui::rgb(0x33d17a).into()
    }

    fn count() -> usize {
        2
    }
}
impl Mytool for TodayBoard {
    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
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
        v_flex().p_4().gap_5().child(Label::new("today"))
    }
}
