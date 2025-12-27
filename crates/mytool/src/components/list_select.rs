use gpui::*;
use gpui_component::select::*;
use serde::{Deserialize, Serialize};

pub fn init(_: &mut App) {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ListSelect {
    name: SharedString,
    code: SharedString,
}

impl ListSelect {
    pub fn letter_prefix(&self) -> char {
        self.name.chars().next().unwrap_or(' ')
    }
}

impl SelectItem for ListSelect {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}
