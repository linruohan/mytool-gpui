use gpui::{Context, IntoElement, Render, Window};
use gpui_component::v_flex;

mod board;

pub struct ProjectStory {
    pub name: String,
    pub description: String,
    pub story: String,
}

impl ProjectStory {
    pub fn new(name: String, description: String, story: String) -> Self {
        Self {
            name,
            description,
            story,
        }
    }
}
impl Render for ProjectStory {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
    }
}
