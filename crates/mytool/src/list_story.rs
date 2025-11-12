use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, ScrollStrategy, Styled, Subscription, Window, actions, px,
};

use crate::{DBState, ItemListDelegate, load_items};
use gpui_component::{
    ActiveTheme, IndexPath, Sizable,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    list::{List, ListDelegate, ListEvent, ListState},
    v_flex,
};
use todos::entity::ItemModel;

actions!(list_story, [SelectedCompany]);

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Rc<ItemModel>>,
    selectable: bool,
    searchable: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Mytool for ListStory {
    fn title() -> &'static str {
        "labels"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ListStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let company_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).searchable(true));

        let _subscriptions =
            vec![
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
            let labels = load_items(db.clone()).await;
            let rc_labels: Vec<Rc<ItemModel>> =
                labels.iter().map(|label| Rc::new(label.clone())).collect();
            println!("list_story: len labels: {}", rc_labels.len());
            let _ = cx
                .update_entity(&company_list_clone, |list, cx| {
                    list.delegate_mut().update_items(rc_labels);
                    cx.notify();
                })
                .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            searchable: true,
            selectable: true,
            company_list,
            selected_company: None,
            _subscriptions,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_item() {
            self.selected_company = Some(company);
        }
    }

    fn toggle_selectable(&mut self, selectable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.selectable = selectable;
        self.company_list.update(cx, |list, cx| {
            list.set_selectable(self.selectable, cx);
        })
    }

    fn toggle_searchable(&mut self, searchable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.searchable = searchable;
        self.company_list.update(cx, |list, cx| {
            list.set_searchable(self.searchable, cx);
        })
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
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .outline()
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
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
                        Button::new("scroll-selected")
                            .outline()
                            .child("Scroll to selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_selected_item(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-item")
                            .outline()
                            .child("Scroll to (5, 1)")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        IndexPath::new(1).section(5),
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
                                this.company_list.update(cx, |list, cx| {
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
                        Checkbox::new("selectable")
                            .label("Selectable")
                            .checked(self.selectable)
                            .on_click(cx.listener(|this, check: &bool, window, cx| {
                                this.toggle_selectable(*check, window, cx)
                            })),
                    )
                    .child(
                        Checkbox::new("searchable")
                            .label("Searchable")
                            .checked(self.searchable)
                            .on_click(cx.listener(|this, check: &bool, window, cx| {
                                this.toggle_searchable(*check, window, cx)
                            })),
                    ),
            )
            .child(
                List::new(&self.company_list)
                    .p(px(8.))
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius),
            )
    }
}
