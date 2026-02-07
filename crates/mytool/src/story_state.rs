use gpui::{AnyView, App, SharedString, Window};
use gpui_component::dock::PanelControl;
use serde::{Deserialize, Serialize};

use crate::{CalendarStory, ListStory, TodoStory, stories::Mytool};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString,
}

impl StoryState {
    pub(crate) fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    pub(crate) fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    pub(crate) fn to_story(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        match self.story_klass.to_string().as_str() {
            "CalendarStory" => story!(CalendarStory),
            "ListStory" => story!(ListStory),
            "TodoStory" => story!(TodoStory),
            _ => {
                tracing::error!(
                    "Invalid story klass: {}. Falling back to ListStory",
                    self.story_klass
                );
                story!(ListStory)
            },
        }
    }
}
