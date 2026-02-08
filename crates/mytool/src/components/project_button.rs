use gpui::{
    App, Context, Corner, EventEmitter, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, SharedString, Window, px,
};
use gpui_component::{button::Button, menu::DropdownMenu, v_flex};

use crate::{
    components::dropdown_button::{DropdownEvent, DropdownState},
    create_button_wrapper,
};

#[derive(Clone)]
pub enum ProjectButtonEvent {
    Selected(String),
}

pub struct ProjectButtonState {
    inner: DropdownState<String>,
}

impl EventEmitter<ProjectButtonEvent> for ProjectButtonState {}

impl Focusable for ProjectButtonState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl ProjectButtonState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx) }
    }

    pub fn project_id(&self) -> Option<String> {
        self.inner.selected()
    }

    pub fn set_project(
        &mut self,
        project_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.inner.set_selected(project_id, window, cx);
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
}

impl Render for ProjectButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();
        let selected_id = self.inner.selected.clone();
        let selected_name = selected_id
            .as_ref()
            .and_then(|id| projects.iter().find(|p| p.id == *id))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Inbox".to_string());

        v_flex().on_action(cx.listener(Self::on_action_select)).child(
            Button::new("project")
                .outline()
                .tooltip("select project")
                .label(SharedString::from(selected_name))
                .dropdown_menu_with_anchor(
                    Corner::TopLeft,
                    move |this: gpui_component::menu::PopupMenu, _, _| {
                        let mut this = this.scrollable(true).max_h(px(400.));
                        this = this.menu(
                            SharedString::from("Inbox"),
                            Box::new(DropdownEvent::Selected(String::new())),
                        );
                        for project in projects.iter() {
                            this = this.menu(
                                SharedString::from(project.name.clone()),
                                Box::new(DropdownEvent::Selected(project.id.clone())),
                            );
                        }
                        this.min_w(px(150.))
                    },
                ),
        )
    }
}

create_button_wrapper!(ProjectButton, ProjectButtonState, "item-project");
