use gpui::{Context, Focusable, ParentElement, Render, Window};

use crate::{
    components::drop_btn::{
        DropdownButtonStateTrait, DropdownEvent, DropdownState, render_dropdown_button,
    },
    create_button_wrapper,
};

#[derive(Clone)]
pub enum ProjectButtonEvent {
    Selected(String),
}

pub struct ProjectButtonState {
    inner: DropdownState<String>,
}

impl gpui::EventEmitter<ProjectButtonEvent> for ProjectButtonState {}

impl Focusable for ProjectButtonState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl Render for ProjectButtonState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        render_dropdown_button::<String, Self>(self, window, cx)
    }
}

impl DropdownButtonStateTrait<String> for ProjectButtonState {
    type EventType = ProjectButtonEvent;

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx) }
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
        let DropdownEvent::Selected(project_id) = action;
        self.inner.selected = Some(project_id.clone());
        cx.emit(ProjectButtonEvent::Selected(project_id.clone()));
        cx.notify();
    }

    fn button_id(&self) -> &'static str {
        "project"
    }

    fn tooltip_text(&self) -> &'static str {
        "select project"
    }

    fn selected_display_name(&self, cx: &mut Context<Self>) -> String {
        let selected_id = self.inner.selected.clone();
        selected_id
            .as_ref()
            .and_then(|id| {
                cx.global::<crate::todo_state::ProjectState>().projects.iter().find(|p| p.id == *id)
            })
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Inbox".to_string())
    }

    fn menu_options(&self, cx: &mut Context<Self>) -> Vec<(String, String)> {
        let mut options = vec![("Inbox".to_string(), String::new())];
        let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();
        for project in projects.iter() {
            options.push((project.name.clone(), project.id.clone()));
        }
        options
    }

    fn min_width(&self) -> f32 {
        150.0
    }
}

impl ProjectButtonState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx) }
    }

    pub fn project_id(&self) -> Option<String> {
        self.selected()
    }

    pub fn set_project(
        &mut self,
        project_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected(project_id, window, cx);
    }
}

create_button_wrapper!(ProjectButton, ProjectButtonState, "item-project");
