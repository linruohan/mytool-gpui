use gpui::SharedString;
use gpui_component::searchable_list::SearchableListItem;

/// Select 选项：显示名与取值（id）分离。
#[derive(Clone, PartialEq, Eq)]
pub struct NamedOption {
    pub id: String,
    pub name: SharedString,
}

impl NamedOption {
    pub fn new(id: impl Into<String>, name: impl Into<SharedString>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl SearchableListItem for NamedOption {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// Helper macro to create standard button wrapper components
#[macro_export]
macro_rules! create_button_wrapper {
    ($button_name:ident, $state_name:ident, $button_id:expr) => {
        #[derive(gpui::IntoElement)]
        pub struct $button_name {
            id: gpui::ElementId,
            style: gpui::StyleRefinement,
            size: gpui_component::Size,
            state: gpui::Entity<$state_name>,
        }

        impl gpui_component::Sizable for $button_name {
            fn with_size(mut self, size: impl Into<gpui_component::Size>) -> Self {
                self.size = size.into();
                self
            }
        }

        impl gpui::Focusable for $button_name {
            fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
                self.state.focus_handle(cx)
            }
        }

        impl gpui::Styled for $button_name {
            fn style(&mut self) -> &mut gpui::StyleRefinement {
                &mut self.style
            }
        }

        impl $button_name {
            pub fn new(state: &gpui::Entity<$state_name>) -> Self {
                Self {
                    id: ($button_id, state.entity_id()).into(),
                    state: state.clone(),
                    size: gpui_component::Size::default(),
                    style: gpui::StyleRefinement::default(),
                }
            }
        }

        impl gpui::RenderOnce for $button_name {
            fn render(
                self,
                _window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) -> impl gpui::IntoElement {
                use gpui::{InteractiveElement, Styled};
                use gpui_component::{StyleSized, StyledExt};

                gpui::div()
                    .id(self.id.clone())
                    .track_focus(&gpui::Focusable::focus_handle(&self, cx).tab_stop(true))
                    .flex_none()
                    .relative()
                    .input_text_size(self.size)
                    .refine_style(&self.style)
                    .child(self.state.clone())
            }
        }
    };
}

/// 宏用于简化状态结构体的基础实现
#[macro_export]
macro_rules! impl_button_state_base {
    ($state_name:ident, $event_type:ty) => {
        impl gpui::EventEmitter<$event_type> for $state_name {}

        impl gpui::Focusable for $state_name {
            fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
                self.focus_handle.clone()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_option() {
        let option = NamedOption::new("inbox", "Inbox");
        assert_eq!(option.value(), "inbox");
        assert_eq!(option.title().as_ref(), "Inbox");
    }
}
