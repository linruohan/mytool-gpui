use anyhow;
use gpui::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, RenderOnce, StyleRefinement, Styled, Window, div,
};
use gpui_component::{Sizable, Size, StyleSized, StyledExt as _};
use serde_json;

// Generic dropdown state
pub struct DropdownState<T: Clone + PartialEq + 'static + Send> {
    focus_handle: FocusHandle,
    pub selected: Option<T>,
}

#[derive(Clone)]
pub enum DropdownEvent<T: Clone + PartialEq + 'static + Send> {
    Selected(T),
}

impl<T: Clone + PartialEq + 'static + Send> Action for DropdownEvent<T> {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new((*self).clone())
    }

    fn partial_eq(&self, _other: &dyn Action) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "DropdownEvent"
    }

    fn name_for_type() -> &'static str {
        "DropdownEvent"
    }

    fn build(_: serde_json::Value) -> Result<Box<dyn Action>, anyhow::Error> {
        Err(anyhow::anyhow!("Cannot build DropdownEvent from JSON"))
    }
}

impl<T: Clone + PartialEq + 'static + Send> EventEmitter<DropdownEvent<T>> for DropdownState<T> {}

impl<T: Clone + PartialEq + 'static + Send> Focusable for DropdownState<T> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<T: Clone + PartialEq + 'static + Send> DropdownState<T> {
    pub fn new(_window: &mut Window, cx: &mut Context<impl std::any::Any>) -> Self {
        Self { focus_handle: cx.focus_handle(), selected: None }
    }

    pub fn selected(&self) -> Option<T> {
        self.selected.clone()
    }

    pub fn set_selected(
        &mut self,
        selected: Option<T>,
        _: &mut Window,
        cx: &mut Context<impl std::any::Any>,
    ) {
        self.selected = selected;
        cx.notify();
    }
}

impl<T: Clone + PartialEq + 'static + Send> Render for DropdownState<T> {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

// Helper trait for items that can be displayed in dropdown
pub trait DropdownItem: Clone + PartialEq + 'static + Send {
    fn display_name(&self) -> String;
    fn id(&self) -> String;
}

// Base trait for button components
pub trait BaseButtonComponent: Sized {
    fn id(&self) -> gpui::ElementId;
    fn style(&mut self) -> &mut StyleRefinement;
    fn size(&self) -> Size;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}

// Generic dropdown button component
#[derive(IntoElement)]
pub struct DropdownButton<T: Clone + PartialEq + 'static + Send> {
    id: gpui::ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<DropdownState<T>>,
}

impl<T: Clone + PartialEq + 'static + Send> Sizable for DropdownButton<T> {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl<T: Clone + PartialEq + 'static + Send> Focusable for DropdownButton<T> {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl<T: Clone + PartialEq + 'static + Send> Styled for DropdownButton<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<T: Clone + PartialEq + 'static + Send> DropdownButton<T> {
    pub fn new(state: &Entity<DropdownState<T>>) -> Self {
        Self {
            id: ("dropdown-button", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl<T: Clone + PartialEq + 'static + Send> RenderOnce for DropdownButton<T> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
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
                    .track_focus(&self.focus_handle(cx).tab_stop(true))
                    .flex_none()
                    .relative()
                    .input_text_size(self.size)
                    .refine_style(&self.style)
                    .child(self.state.clone())
            }
        }
    };
}
