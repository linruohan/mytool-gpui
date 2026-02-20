use std::sync::Arc;

use gpui::{Context, EventEmitter, Focusable, ParentElement, Render, Window};
use todos::entity::SectionModel;

use crate::{
    create_button_wrapper,
    todo_state::TodoStore,
    ui::components::drop_btn::{
        DropdownButtonStateTrait, DropdownEvent, DropdownState, render_dropdown_button,
    },
};

#[derive(Clone)]
pub enum SectionEvent {
    Selected(String),
}

pub struct SectionState {
    inner: DropdownState<String>,
    pub sections: Option<Vec<Arc<SectionModel>>>,
}

impl EventEmitter<SectionEvent> for SectionState {}

impl Focusable for SectionState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl Render for SectionState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        render_dropdown_button::<String, Self>(self, window, cx)
    }
}

impl DropdownButtonStateTrait<String> for SectionState {
    type EventType = SectionEvent;

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx), sections: None }
    }

    fn inner(&self) -> &DropdownState<String> {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut DropdownState<String> {
        &mut self.inner
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

    fn button_id(&self) -> &'static str {
        "section"
    }

    fn tooltip_text(&self) -> &'static str {
        "select section"
    }

    fn selected_display_name(&self, cx: &mut Context<Self>) -> String {
        let sections =
            self.sections.clone().unwrap_or_else(|| cx.global::<TodoStore>().sections.clone());
        let selected_id = self.inner.selected.clone();
        selected_id
            .as_ref()
            .and_then(|id| sections.iter().find(|s| s.id == *id))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "No Section".to_string())
    }

    fn menu_options(&self, cx: &mut Context<Self>) -> Vec<(String, String)> {
        let mut options = vec![("No Section".to_string(), String::new())];
        let sections =
            self.sections.clone().unwrap_or_else(|| cx.global::<TodoStore>().sections.clone());
        for section in sections.iter() {
            options.push((section.name.clone(), section.id.clone()));
        }
        options
    }

    fn min_width(&self) -> f32 {
        150.0
    }
}

impl SectionState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx), sections: None }
    }

    pub fn section_id(&self) -> Option<String> {
        self.selected()
    }

    pub fn set_section(
        &mut self,
        section_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected(section_id, window, cx);
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
}

create_button_wrapper!(SectionButton, SectionState, "item-section");
