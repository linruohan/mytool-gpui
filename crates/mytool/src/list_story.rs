use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext, Context, Edges, ElementId,
    Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    RenderOnce, Styled, Task, Window,
};

use crate::{get_projects, DBState};
use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, Selectable,
};
use todos::entity::ProjectModel;

actions!(list_story, [SelectedProject]);

#[derive(IntoElement)]
struct ProjectListItem {
    base: ListItem,
    project: ProjectModel,
    selected: bool,
}

impl ProjectListItem {
    pub fn new(id: impl Into<ElementId>, project: ProjectModel, selected: bool) -> Self {
        ProjectListItem {
            project,
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

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
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
                            .child(Label::new(self.project.name.clone()).whitespace_nowrap()),
                    )
                    .child(
                        div().text_sm().overflow_x_hidden().child(
                            Label::new(self.project.color.unwrap_or_default())
                                .whitespace_nowrap()
                                .text_color(text_color.opacity(0.5)),
                        ),
                    ),
            )
    }
}

struct ProjectMenuListDelegate {
    menus: Vec<ProjectModel>,
    matched_menus: Vec<ProjectModel>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    loading: bool,
    query: String,
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
            return Some(ProjectListItem::new(ix, company.clone(), selected));
        }

        None
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
        if let Some(selected) = self.selected_index {
            if let Some(menu) = self.matched_menus.get(selected) {
                println!("Selected menu: {}", menu.name);
            }
        }
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedProject), cx);
    }
}

impl ProjectMenuListDelegate {
    fn new() -> Self {
        Self {
            menus: vec![],
            matched_menus: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
        }
    }
    fn update_menus(&mut self, menus: Vec<ProjectModel>) {
        self.menus = menus;
        self.matched_menus = self.menus.clone();
        if !self.matched_menus.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(0);
        }
    }
    fn selected_company(&self) -> Option<ProjectModel> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.menus.get(ix).cloned()
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    menu_list: Entity<List<ProjectMenuListDelegate>>,
    selected_menu: Option<ProjectModel>,
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
        let company_list = cx.new(|cx| {
            List::new(ProjectMenuListDelegate::new(), window, cx).paddings(Edges::all(px(8.)))
        });

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

        let company_list_clone = company_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let projects = get_projects(db.clone()).await;
            let _ = cx.update_entity(&company_list_clone, |list, cx| {
                list.delegate_mut().update_menus(projects);
                cx.notify();
            });
        })
        .detach();
        Self {
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
