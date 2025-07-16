use gpui::{
    actions, div, px, App, Context, ElementId, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Task, Timer, Window,
};
use std::time::Duration;

use fake::Fake;
use gpui_component::{
    h_flex, hsl,
    label::Label,
    list::{List, ListDelegate, ListItem},
    v_flex, ActiveTheme,
};

actions!(mytool, [SelectedProject]);

#[derive(Clone, Default)]
pub struct Project {
    name: SharedString,
    industry: SharedString,
    last_done: f64,
    prev_close: f64,

    change_percent: f64,
    change_percent_str: SharedString,
    last_done_str: SharedString,
    prev_close_str: SharedString,
    // description: String,
}

impl Project {
    fn prepare(mut self) -> Self {
        self.change_percent = (self.last_done - self.prev_close) / self.prev_close;
        self.change_percent_str = format!("{:.2}%", self.change_percent).into();
        self.last_done_str = format!("{:.2}", self.last_done).into();
        self.prev_close_str = format!("{:.2}", self.prev_close).into();
        self
    }

    pub fn random_update(&mut self) {
        self.last_done = self.prev_close * (1.0 + (-0.2..0.2).fake::<f64>());
    }
}

#[derive(IntoElement)]
pub struct ProjectListItem {
    base: ListItem,
    ix: usize,
    project: Project,
    selected: bool,
}

impl ProjectListItem {
    pub fn new(id: impl Into<ElementId>, project: Project, ix: usize, selected: bool) -> Self {
        ProjectListItem {
            project,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl RenderOnce for ProjectListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let trend_color = match self.project.change_percent {
            change if change > 0.0 => hsl(0.0, 79.0, 53.0),
            change if change < 0.0 => hsl(100.0, 79.0, 53.0),
            _ => cx.theme().foreground,
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix.is_multiple_of(2) {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        self.base
            .px_3()
            .py_1()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        v_flex()
                            .gap_1()
                            .max_w(px(500.))
                            .overflow_x_hidden()
                            .flex_nowrap()
                            .child(Label::new(self.project.name.clone()).whitespace_nowrap())
                            .child(
                                div().text_sm().overflow_x_hidden().child(
                                    Label::new(self.project.industry.clone())
                                        .whitespace_nowrap()
                                        .text_color(text_color.opacity(0.5)),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(px(65.))
                                    .text_color(text_color)
                                    .child(self.project.last_done_str.clone()),
                            )
                            .child(
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .px_1()
                                        .text_color(trend_color)
                                        .child(self.project.change_percent_str.clone()),
                                ),
                            ),
                    ),
            )
    }
}

pub struct ProjectListDelegate {
    pub projects: Vec<Project>,
    pub matched_projects: Vec<Project>,
    pub selected_index: Option<usize>,
    pub confirmed_index: Option<usize>,
    pub query: String,
    pub loading: bool,
    pub eof: bool,
}

impl ListDelegate for ProjectListDelegate {
    type Item = ProjectListItem;

    fn items_count(&self, _: &App) -> usize {
        self.matched_projects.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_projects = self
            .projects
            .iter()
            .filter(|project| project.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedProject), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_item(
        &self,
        ix: usize,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(project) = self.matched_projects.get(ix) {
            return Some(ProjectListItem::new(ix, project.clone(), ix, selected));
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut()
                    .projects
                    .extend((0..3).map(|_| random_project()));
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().projects.len() >= 3;
            });
        })
        .detach();
    }
}

impl ProjectListDelegate {
    pub fn selected_project(&self) -> Option<Project> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.projects.get(ix).cloned()
    }
}

pub fn random_project() -> Project {
    let last_done = (0.0..6.0).fake::<f64>();
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    Project {
        name: fake::faker::company::en::CompanyName()
            .fake::<String>()
            .into(),
        industry: fake::faker::company::en::Industry().fake::<String>().into(),
        last_done,
        prev_close,
        ..Default::default()
    }
    .prepare()
}
