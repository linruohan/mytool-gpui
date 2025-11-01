use gpui::{
    App, Context, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Task,
    Window, actions,
};
use gpui_component::{
    IndexPath, Selectable,
    label::Label,
    list::{List, ListDelegate, ListItem},
    v_flex,
};
use std::rc::Rc;
use todos::entity::ProjectModel;

actions!(project, [SelectedProject]);

pub enum ProjectEvent {
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
        ProjectListItem {
            project,
            ix,
            base: ListItem::new(id),
            selected,
        }
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
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(Label::new(self.project.name.clone()))
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
    fn search_project(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let projects: Vec<Rc<ProjectModel>> = self
            ._projects
            .iter()
            .filter(|project| {
                project
                    .name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for project in projects.into_iter() {
            self.matched_projects.push(vec![project]);
        }
    }
    #[allow(dead_code)]
    pub fn update_projects(&mut self, projects: Vec<Rc<ProjectModel>>) {
        self._projects = projects;
        self.matched_projects = vec![self._projects.clone()];
        if !self.matched_projects.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }
    pub fn add(&mut self, project: Rc<ProjectModel>) {
        let mut projects = self._projects.clone();

        projects.push(project);
        self.update_projects(projects);
    }
    #[allow(dead_code)]
    pub fn selected_project(&self) -> Option<Rc<ProjectModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_projects
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
}

impl ListDelegate for ProjectListDelegate {
    type Item = ProjectListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.search_project(query.to_owned());
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _app: &App) -> usize {
        self.matched_projects.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(menu) = self.matched_projects[ix.section].get(ix.row) {
            return Some(ProjectListItem::new(ix, menu.clone(), ix, selected));
        }

        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedProject), cx);
    }
}
