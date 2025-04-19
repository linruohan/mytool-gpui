use std::time::Duration;

use fake::Fake;
use gpui::{
    actions, div, px, App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, SharedString, Styled,
    Subscription, Task, Timer, Window,
};

use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    h_flex, hsl,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, Sizable,
};

actions!(list_story, [SelectedProject]);

#[derive(Clone, Default)]
struct Project {
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

    fn random_update(&mut self) {
        self.last_done = self.prev_close * (1.0 + (-0.2..0.2).fake::<f64>());
    }
}

#[derive(IntoElement)]
struct ProjectListItem {
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
        } else if self.ix % 2 == 0 {
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

struct ProjectListDelegate {
    projects: Vec<Project>,
    matched_projects: Vec<Project>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
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
    fn selected_project(&self) -> Option<Project> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.projects.get(ix).cloned()
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    project_list: Entity<List<ProjectListDelegate>>,
    selected_project: Option<Project>,
    _subscriptions: Vec<Subscription>,
}

impl super::Mytool for ListStory {
    fn title() -> &'static str {
        "List"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ListStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let projects = (0..3).map(|_| random_project()).collect::<Vec<Project>>();

        let delegate = ProjectListDelegate {
            matched_projects: projects.clone(),
            projects,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let project_list = cx.new(|cx| List::new(delegate, window, cx));
        // project_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });
        let _subscriptions =
            vec![
                cx.subscribe(&project_list, |_, _, ev: &ListEvent, _| match ev {
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.project_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        .projects
                        .iter_mut()
                        .for_each(|project| {
                            project.random_update();
                        });
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            project_list,
            selected_project: None,
            _subscriptions,
        }
    }

    fn selected_project(&mut self, _: &SelectedProject, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.project_list.read(cx);
        if let Some(project) = picker.delegate().selected_project() {
            self.selected_project = Some(project);
        }
    }
}

fn random_project() -> Project {
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

impl Focusable for ListStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_project))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    list.scroll_to_item(0, window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        list.delegate().items_count(cx) - 1,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-selected")
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(selected, window, cx);
                                    }
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.project_list.read(cx).delegate().loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.project_list.update(cx, |this, cx| {
                                    this.delegate_mut().loading = *check;
                                    cx.notify();
                                })
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.project_list.clone()),
            )
    }
}
