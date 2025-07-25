use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window,
};

use gpui_component::{dock::PanelControl, label::Label, v_flex, IconName};

use crate::{Board, Mytool};

pub struct ProjectItem {
    focus_handle: FocusHandle,
}

impl ProjectItem {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}
impl Board for ProjectItem {
    fn icon() -> IconName {
        IconName::ProcessErrorSymbolic
    }

    fn color() -> Hsla {
        gpui::rgb(0x33D17A).into()
    }

    fn count() -> usize {
        2
    }
}

impl Mytool for ProjectItem {
    fn title() -> &'static str {
        "Project"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for ProjectItem {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProjectItem {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex().p_4().gap_5().child(Label::new("project"))
    }
}
