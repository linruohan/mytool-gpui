use std::rc::Rc;

use gpui::{
    App, Context, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Task, Window, actions, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Selectable, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use todos::entity::ProjectModel;

actions!(project, [SelectedProject]);
pub enum ProjectEvent {
    Loaded,
    Added(Rc<ProjectModel>),
    Modified(Rc<ProjectModel>),
    Deleted(Rc<ProjectModel>),
}

#[derive(IntoElement)]
pub struct ProjectListItem {
    base: ListItem,
    ix: IndexPath,
    project: Rc<ProjectModel>,
    selected: bool,
}

impl ProjectListItem {
    pub fn new(
        id: impl Into<ElementId>,
        project: Rc<ProjectModel>,
        ix: IndexPath,
        selected: bool,
    ) -> Self {
        ProjectListItem { project, ix, base: ListItem::new(id), selected }
    }
}

impl Selectable for ProjectListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for ProjectListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .when(self.selected, |this| this.border_color(cx.theme().list_active_border))
            .rounded(cx.theme().radius)
            .child(
                h_flex().items_center().justify_between().gap_2().text_color(text_color).child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .justify_end()
                        .child(div().w(px(15.)).child(self.project.id.clone()))
                        .child(div().w(px(120.)).child(self.project.name.clone()))
                        .child(
                            div().w(px(115.)).child(
                                self.project.description.clone().unwrap_or_default().clone(),
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .px_2()
                                .gap_2()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(
                                    Button::new("edit")
                                        .small()
                                        .ghost()
                                        .compact()
                                        .icon(IconName::EditSymbolic)
                                        .on_click(move |_event, _window, _cx| {
                                            let project = self.project.clone();
                                            println!("edit project:{:?}", project);
                                        }),
                                )
                                .child(
                                    Button::new("delete")
                                        .icon(IconName::UserTrashSymbolic)
                                        .small()
                                        .ghost()
                                        .on_click(|_, _, _cx| {
                                            println!("delete project:");
                                        }),
                                ),
                        ),
                ),
            )
    }
}

pub struct ProjectListDelegate {
    pub _projects: Vec<Rc<ProjectModel>>,
    pub matched_projects: Vec<Vec<Rc<ProjectModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl ProjectListDelegate {
    pub fn new() -> Self {
        Self {
            _projects: vec![],
            matched_projects: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let projects: Vec<Rc<ProjectModel>> = self
            ._projects
            .iter()
            .filter(|project| project.name.to_lowercase().contains(&self.query.to_lowercase()))
            .cloned()
            .collect();
        for project in projects.into_iter() {
            self.matched_projects.push(vec![project]);
        }
    }

    pub fn update_projects(&mut self, projects: Vec<Rc<ProjectModel>>) {
        self._projects = projects;
        self.matched_projects = vec![self._projects.clone()];
        if !self.matched_projects.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }

    pub fn selected_project(&self) -> Option<Rc<ProjectModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_projects.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }
}
impl ListDelegate for ProjectListDelegate {
    type Item = ProjectListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_projects[section].len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(project) = self.matched_projects[ix.section].get(ix.row) {
            return Some(ProjectListItem::new(ix, project.clone(), ix, selected));
        }

        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedProject), cx);
    }
}
