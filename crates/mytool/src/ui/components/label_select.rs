use std::sync::Arc;

use gpui::{App, IntoElement, ParentElement, SharedString, Window};
use gpui_component::{checkbox::Checkbox, h_flex, select::SelectItem};
use serde::{Deserialize, Serialize};
use todos::entity::LabelModel;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
struct LabelSelect {
    label: Arc<LabelModel>,
    selected: bool,
    pub checked: bool,
}
#[allow(dead_code)]
impl LabelSelect {
    fn new(label: Arc<LabelModel>, checked: bool) -> Self {
        Self { label, selected: false, checked }
    }

    fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    fn name(&self) -> String {
        self.label.name.clone()
    }
}
impl SelectItem for LabelSelect {
    type Value = Arc<LabelModel>;

    fn title(&self) -> SharedString {
        self.label.name.clone().into()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(format!("{} ", self.label.name.clone()).into_any_element())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex().child(Checkbox::new("is").checked(self.checked)).child(self.label.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.label
    }
}
