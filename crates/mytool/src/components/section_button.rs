use std::sync::Arc;

use gpui::{
    App, Context, Corner, EventEmitter, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, SharedString, Window, px,
};
use gpui_component::{button::Button, menu::DropdownMenu, v_flex};
use todos::entity::SectionModel;

use crate::{
    components::dropdown_button::{DropdownEvent, DropdownState},
    create_button_wrapper,
};

pub enum SectionEvent {
    Selected(String),
}

pub struct SectionState {
    inner: DropdownState<String>,
    pub sections: Option<Vec<Arc<SectionModel>>>,
}

impl EventEmitter<SectionEvent> for SectionState {}

impl Focusable for SectionState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl SectionState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(_window, cx), sections: None }
    }

    pub fn section_id(&self) -> Option<String> {
        self.inner.selected()
    }

    pub fn set_section(
        &mut self,
        section_id: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.inner.set_selected(section_id, _window, cx);
    }

    pub fn set_sections(
        &mut self,
        sections: Option<Vec<Arc<SectionModel>>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sections = sections;
        self.inner.selected = None;
        cx.notify();
    }

    fn on_action_select(
        &mut self,
        action: &DropdownEvent<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DropdownEvent::Selected(section_id) = action;
        self.inner.selected = Some(section_id.clone());
        cx.emit(SectionEvent::Selected(section_id.clone()));
        cx.notify();
    }
}

impl Render for SectionState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let sections = self
            .sections
            .clone()
            .unwrap_or_else(|| cx.global::<crate::todo_state::SectionState>().sections.clone());
        let selected_id = self.inner.selected.clone();
        let selected_name = selected_id
            .as_ref()
            .and_then(|id| sections.iter().find(|s| s.id == *id))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "No Section".to_string());

        v_flex().on_action(cx.listener(Self::on_action_select)).child(
            Button::new("section")
                .outline()
                .tooltip("select section")
                .label(SharedString::from(selected_name))
                .dropdown_menu_with_anchor(
                    Corner::TopLeft,
                    move |this: gpui_component::menu::PopupMenu, _, _| {
                        let mut this = this.scrollable(true).max_h(px(400.));
                        this = this.menu(
                            SharedString::from("No Section"),
                            Box::new(DropdownEvent::Selected(String::new())),
                        );
                        for section in sections.iter() {
                            this = this.menu(
                                SharedString::from(section.name.clone()),
                                Box::new(DropdownEvent::Selected(section.id.clone())),
                            );
                        }
                        this.min_w(px(150.))
                    },
                ),
        )
    }
}

create_button_wrapper!(SectionButton, SectionState, "item-section");
