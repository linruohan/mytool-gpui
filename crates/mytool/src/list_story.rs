use std::rc::Rc;

use gpui::{actions, div, prelude::FluentBuilder, px, App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window};
use gpui_component::{h_flex, list::{List, ListEvent, ListState}, v_flex, ActiveTheme, Colorize, Sizable};
use itertools::Itertools;
use todos::entity::ItemModel;

use crate::{
    load_items, section, ColorGroup, ColorGroupEvent, ColorGroupState, DBState, ItemInfo,
    ItemInfoEvent, ItemInfoState, ItemListDelegate,
};

actions!(list_story, [SelectedCompany]);

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Rc<ItemModel>>,
    selectable: bool,
    searchable: bool,
    _subscriptions: Vec<Subscription>,
    item_info: Entity<ItemInfoState>,
    item: Rc<ItemModel>,
    color: Entity<ColorGroupState>,
    selected_color: Option<Hsla>,
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
        let color = cx.new(|cx| ColorGroupState::new(window, cx).default_value(cx.theme().primary));

        let item_info = cx.new(|cx| {
            let picker = ItemInfoState::new(window, cx);
            picker
        });
        let _subscriptions = vec![
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                    println!("Color changed to: {:?}", color);
                },
            }),
            cx.subscribe(&item_info, |this, _, event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |item_info, cx| {
                    item_info.handel_item_info_event(event, cx);
                });
            }),
            cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
                ListEvent::Select(ix) => {
                    println!("List Selected: {:?}", ix);
                },
                ListEvent::Confirm(ix) => {
                    println!("List Confirmed: {:?}", ix);
                },
                ListEvent::Cancel => {
                    println!("List Cancelled");
                },
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
            color,
            selected_color: Some(cx.theme().primary),
            focus_handle: cx.focus_handle(),
            searchable: true,
            selectable: true,
            company_list,
            selected_company: None,
            _subscriptions,
            item_info,
            item: Rc::new(ItemModel::default()),
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
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.color.read(cx).focus_handle(cx)
        // self.focus_handle.clone()
    }
}
impl Render for ListStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(section("item_info").child(ItemInfo::new(&self.item_info)))
            .child(ColorGroup::new(&self.color).large())
            .when_some(self.selected_color, |this, color| {
                this.child(
                    h_flex().gap_4().child(div()
                        .id(SharedString::from(format!("color-{}", color.to_hex())))
                        .h_5()
                        .w_5()
                        .bg(color)
                        .border_1()
                        .border_color(color.darken(0.1)))
                            .child(color.to_hex())
                )
            })
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
