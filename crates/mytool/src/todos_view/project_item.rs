use gpui::{App, AppContext, Entity, ParentElement, Render, SharedString, Styled, Window};

use gpui_component::{label::Label, v_flex};

#[derive(Debug, Clone)]
pub struct ProjectItem {
    pub name: SharedString,
}

impl ProjectItem {
    pub fn view(name: SharedString, _window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self::new(name))
    }

    pub(crate) fn new(name: SharedString) -> Self {
        Self { name }
    }
}

impl Render for ProjectItem {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex().p_4().gap_5().child(Label::new(self.name.clone()))
    }
}
