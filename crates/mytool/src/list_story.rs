use std::sync::Arc;
use std::time::Duration;

use fake::Fake;
use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext, Context, Edges, ElementId,
    Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    RenderOnce, SharedString, Styled, Task, Timer, Window,
};

use crate::DBState;
use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, Selectable,
};
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

actions!(list_story, [SelectedProject]);

#[derive(Clone, Default)]
struct ProjectMenu {
    name: SharedString,
    color: SharedString,
}

#[derive(IntoElement)]
struct ProjectListItem {
    base: ListItem,
    ix: usize,
    menu: ProjectMenu,
    selected: bool,
}

impl ProjectListItem {
    pub fn new(id: impl Into<ElementId>, menu: ProjectMenu, ix: usize, selected: bool) -> Self {
        ProjectListItem {
            menu,
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix.is_multiple_of(2) {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .bg(bg_color)
            .border_1()
            .border_color(bg_color)
            .when(self.selected, |this| {
                this.border_color(cx.theme().list_active_border)
            })
            .rounded(cx.theme().radius)
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
                            .child(Label::new(self.menu.name.clone()).whitespace_nowrap()),
                    )
                    .child(
                        div().text_sm().overflow_x_hidden().child(
                            Label::new(self.menu.color.clone())
                                .whitespace_nowrap()
                                .text_color(text_color.opacity(0.5)),
                        ),
                    ),
            )
    }
}

struct ProjectMenuListDelegate {
    menus: Vec<ProjectMenu>,
    matched_menus: Vec<ProjectMenu>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
}

impl ListDelegate for ProjectMenuListDelegate {
    type Item = ProjectListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_menus = self
            .menus
            .iter()
            .filter(|menu| menu.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    fn items_count(&self, _: &App) -> usize {
        self.matched_menus.len()
    }

    fn render_item(
        &self,
        ix: usize,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_menus.get(ix) {
            return Some(ProjectListItem::new(ix, company.clone(), ix, selected));
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
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

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedProject), cx);
    }

    fn can_load_more(&self, _: &App) -> bool {
        false
    }

    fn load_more_threshold(&self) -> usize {
        10
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut()
                    .menus
                    .extend((0..2).map(|_| random_company()));
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().menus.len() >= 15;
            });
        })
        .detach();
    }
}

impl ProjectMenuListDelegate {
    fn selected_company(&self) -> Option<ProjectMenu> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.menus.get(ix).cloned()
    }
}

pub struct ListStory {
    db: Arc<Mutex<DatabaseConnection>>,
    focus_handle: FocusHandle,
    menu_list: Entity<List<ProjectMenuListDelegate>>,
    selected_menu: Option<ProjectMenu>,
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
        let companies = (0..10)
            .map(|_| random_company())
            .collect::<Vec<ProjectMenu>>();

        let delegate = ProjectMenuListDelegate {
            matched_menus: companies.clone(),
            menus: companies,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let company_list =
            cx.new(|cx| List::new(delegate, window, cx).paddings(Edges::all(px(8.))));

        let _subscriptions = [
            cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
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

        let db = cx.global::<DBState>().conn.clone();
        Self {
            db,
            focus_handle: cx.focus_handle(),
            menu_list: company_list,
            selected_menu: None,
        }
    }

    fn selected_company(&mut self, _: &SelectedProject, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.menu_list.read(cx);
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_menu = Some(company);
        }
    }
}

fn random_company() -> ProjectMenu {
    let last_done = (0.0..999.0).fake::<f64>();
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    ProjectMenu {
        name: fake::faker::company::en::CompanyName()
            .fake::<String>()
            .into(),
        color: fake::faker::company::en::Industry().fake::<String>().into(),
        ..Default::default()
    }
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
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(
                h_flex().gap_2().flex_wrap().child(
                    Checkbox::new("loading")
                        .label("Loading")
                        .checked(self.menu_list.read(cx).delegate().loading)
                        .on_click(cx.listener(|this, check: &bool, _, cx| {
                            this.menu_list.update(cx, |this, cx| {
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
                    .child(self.menu_list.clone()),
            )
    }
}
