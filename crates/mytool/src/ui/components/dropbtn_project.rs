use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, ParentElement, Render, Styled,
    Subscription, Window, px,
};
use gpui_component::{
    IndexPath, Sizable,
    searchable_list::SearchableVec,
    select::{Select, SelectEvent, SelectState},
};

use crate::{create_button_wrapper, todo_state::TodoStore, ui::components::drop_btn::NamedOption};

#[derive(Clone)]
pub enum ProjectButtonEvent {
    Selected(String),
}

pub struct ProjectButtonState {
    select: Entity<SelectState<SearchableVec<NamedOption>>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ProjectButtonEvent> for ProjectButtonState {}

impl Focusable for ProjectButtonState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.select.focus_handle(cx)
    }
}

impl Render for ProjectButtonState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.sync_items(window, cx);
        Select::new(&self.select)
            .small()
            .appearance(true)
            .placeholder("收件箱")
            .search_placeholder("搜索项目")
            .w(px(150.))
    }
}

impl ProjectButtonState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(Self::options(cx)),
                Some(IndexPath::default()),
                window,
                cx,
            )
            .searchable(true)
        });

        let _subscriptions = vec![cx.subscribe(
            &select,
            |_, _, event: &SelectEvent<SearchableVec<NamedOption>>, cx| {
                if let SelectEvent::Confirm(Some(project_id)) = event {
                    cx.emit(ProjectButtonEvent::Selected(project_id.clone()));
                }
            },
        )];

        Self { select, _subscriptions }
    }

    fn options(cx: &App) -> Vec<NamedOption> {
        let mut options = vec![NamedOption::new(String::new(), "收件箱")];
        for project in cx.global::<TodoStore>().projects.iter() {
            options.push(NamedOption::new(project.id.clone(), project.name.clone()));
        }
        options
    }

    fn sync_items(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = Self::options(cx);
        self.select.update(cx, |select, cx| {
            select.set_items(SearchableVec::new(items), window, cx);
        });
    }

    pub fn project_id(&self, cx: &App) -> Option<String> {
        self.select.read(cx).selected_value().cloned()
    }

    pub fn set_project(
        &mut self,
        project_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = project_id.unwrap_or_default();
        self.select.update(cx, |select, cx| {
            select.set_selected_value(&id, window, cx);
        });
    }
}

create_button_wrapper!(ProjectButton, ProjectButtonState, "item-project");
