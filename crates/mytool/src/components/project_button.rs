use std::rc::Rc;

use gpui::{
    Action, App, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyleSized, StyledExt as _, button::Button, menu::DropdownMenu, v_flex,
};
use serde::Deserialize;
use todos::entity::ProjectModel;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = project_button, no_json)]
struct ProjectButtonInfo(String);

pub enum ProjectButtonEvent {
    Selected(String),
}

pub struct ProjectButtonState {
    focus_handle: FocusHandle,
    pub selected_project: Option<Rc<ProjectModel>>,
}

impl EventEmitter<ProjectButtonEvent> for ProjectButtonState {}

impl Focusable for ProjectButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ProjectButtonState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), selected_project: None }
    }

    pub fn project(&self) -> Option<Rc<ProjectModel>> {
        self.selected_project.clone()
    }

    pub fn set_project(
        &mut self,
        project: Option<Rc<ProjectModel>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_project = project;
        cx.notify()
    }

    fn on_action_info(
        &mut self,
        info: &ProjectButtonInfo,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 根据project_id更新selected_project
        if info.0.is_empty() {
            // 选择Inbox
            self.selected_project = None;
        } else {
            // 根据project_id查找project
            let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();
            if let Some(project) = projects.iter().find(|p| &p.id == &info.0) {
                self.selected_project = Some(project.clone());
            }
        }
        cx.emit(ProjectButtonEvent::Selected(info.0.clone()));
        cx.notify();
    }
}

/// A ProjectButton element.
#[derive(IntoElement)]
pub struct ProjectButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ProjectButtonState>,
    projects: Vec<Rc<ProjectModel>>,
}

impl Sizable for ProjectButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ProjectButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ProjectButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for ProjectButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();

        v_flex().on_action(cx.listener(Self::on_action_info)).child(
            Button::new("project")
                .outline()
                .tooltip("select project")
                .icon(IconName::Inbox)
                .label(
                    self.selected_project
                        .as_ref()
                        .map(|p| SharedString::from(p.name.clone()))
                        .unwrap_or_else(|| SharedString::from("Inbox")),
                )
                .dropdown_menu_with_anchor(Corner::TopLeft, move |this, _, _| {
                    let mut this = this.scrollable(true).max_h(px(400.));

                    // Add "Inbox" option (no project)
                    this = this.menu(
                        SharedString::from("Inbox"),
                        Box::new(ProjectButtonInfo(String::new())),
                    );

                    // Add all projects
                    for project in projects.iter() {
                        let project_id = project.id.clone();
                        this = this.menu(
                            SharedString::from(project.name.clone()),
                            Box::new(ProjectButtonInfo(project_id)),
                        )
                    }
                    this.min_w(px(150.))
                }),
        )
    }
}

impl ProjectButton {
    /// Create a new ProjectButton with the given [`ProjectButtonState`].
    pub fn new(state: &Entity<ProjectButtonState>, projects: Vec<Rc<ProjectModel>>) -> Self {
        Self {
            id: ("item-project", state.entity_id()).into(),
            state: state.clone(),
            projects,
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ProjectButton {
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
