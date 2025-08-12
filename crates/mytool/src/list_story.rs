use std::rc::Rc;

use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext, Context, Edges, ElementId,
    Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    RenderOnce, ScrollStrategy, SharedString, Styled, Subscription, Task, Window,
};

use gpui_component::{
    button::Button,
    h_flex,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, IndexPath, Selectable, Sizable,
};
use todos::entity::ProjectModel;

use crate::{get_projects, DBState};

actions!(list_story, [SelectedMenu]);

#[derive(IntoElement)]
struct MenuListItem {
    base: ListItem,
    ix: IndexPath,
    menu: Rc<ProjectModel>,
    selected: bool,
}

impl MenuListItem {
    pub fn new(
        id: impl Into<ElementId>,
        menu: Rc<ProjectModel>,
        ix: IndexPath,
        selected: bool,
    ) -> Self {
        MenuListItem {
            menu,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl Selectable for MenuListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for MenuListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };
        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix.row.is_multiple_of(2) {
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
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(px(65.))
                                    .text_color(text_color)
                                    .child(self.menu.id.clone()),
                            )
                            .child(
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .px_1()
                                        .child(self.menu.name.clone()),
                                ),
                            ),
                    ),
            )
    }
}

struct MenuListDelegate {
    _menus: Vec<Rc<ProjectModel>>,
    matched_menus: Vec<Vec<Rc<ProjectModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl MenuListDelegate {
    fn new() -> Self {
        Self {
            _menus: vec![],
            matched_menus: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }
    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let companies: Vec<Rc<ProjectModel>> = self
            ._menus
            .iter()
            .filter(|menu| {
                menu.name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for menu in companies.into_iter() {
            self.matched_menus.push(vec![menu]);
        }
    }
    fn update_menus(&mut self, menus: Vec<Rc<ProjectModel>>) {
        self._menus = menus;
        self.matched_menus = vec![self._menus.clone()];
        if !self.matched_menus.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::new(0));
        }
    }
    fn selected_menu(&self) -> Option<Rc<ProjectModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_menus
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
}

impl ListDelegate for MenuListDelegate {
    type Item = MenuListItem;
    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_menus[section].len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedMenu), cx);
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

    fn render_item(
        &self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(menu) = self.matched_menus[ix.section].get(ix.row) {
            return Some(MenuListItem::new(ix, menu.clone(), ix, selected));
        }

        None
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    menu_list: Entity<List<MenuListDelegate>>,
    selected_menu: Option<Rc<ProjectModel>>,
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
        let menu_list = cx
            .new(|cx| List::new(MenuListDelegate::new(), window, cx).paddings(Edges::all(px(8.))));

        let _subscriptions = vec![
            cx.subscribe(&menu_list, |_, _, ev: &ListEvent, _| match ev {
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
        let company_list_clone = menu_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let projects = get_projects(db.clone()).await;
            let rc_projects: Vec<Rc<ProjectModel>> =
                projects.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("get rc_projects:{}", rc_projects.len());
            let _ = cx.update_entity(&company_list_clone, |list, cx| {
                list.delegate_mut().update_menus(rc_projects);
                cx.notify();
            });
        })
        .detach();
        Self {
            focus_handle: cx.focus_handle(),
            menu_list,
            selected_menu: None,
            _subscriptions,
        }
    }

    fn selected_menu(&mut self, _: &SelectedMenu, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.menu_list.read(cx);
        if let Some(menu) = picker.delegate().selected_menu() {
            self.selected_menu = Some(menu);
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
            .on_action(cx.listener(Self::selected_menu))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .outline()
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.menu_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        IndexPath::default(),
                                        ScrollStrategy::Top,
                                        window,
                                        cx,
                                    );
                                    cx.notify();
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-center")
                            .outline()
                            .child("Scroll to section 2")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.menu_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        IndexPath::default().section(1).row(0),
                                        ScrollStrategy::Center,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .outline()
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.menu_list.update(cx, |list, cx| {
                                    let last_section =
                                        list.delegate().sections_count(cx).saturating_sub(1);

                                    list.scroll_to_item(
                                        IndexPath::default().section(last_section).row(
                                            list.delegate()
                                                .items_count(last_section, cx)
                                                .saturating_sub(1),
                                        ),
                                        ScrollStrategy::Top,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-selected")
                            .outline()
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.menu_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(
                                            selected,
                                            ScrollStrategy::Top,
                                            window,
                                            cx,
                                        );
                                    }
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
