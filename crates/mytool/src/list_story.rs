use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, actions, div,
    prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Colorize, Sizable,
    divider::Divider,
    h_flex,
    list::{ListEvent, ListState},
    v_flex,
};
use todos::entity::ItemModel;

use crate::{
    ColorGroup, ColorGroupEvent, ColorGroupState, DBState, ItemInfo, ItemInfoEvent, ItemInfoState,
    ItemListDelegate, LabelsPicker, LabelsPickerEvent, LabelsPickerState, LabelsPopoverEvent,
    LabelsPopoverList, load_items, popover_list::PopoverList, section,
};

actions!(list_story, [SelectedCompany]);

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Rc<ItemModel>>,
    _subscriptions: Vec<Subscription>,
    item_info: Entity<ItemInfoState>,
    color: Entity<ColorGroupState>,
    selected_color: Option<Hsla>,
    pub popover_list: Entity<PopoverList>,
    pub label_popover_list: Entity<LabelsPopoverList>,
    label_picker: Entity<LabelsPickerState>,
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
        let label_picker = cx.new(|cx| LabelsPickerState::new(window, cx));
        let popover_list = cx.new(|cx| PopoverList::new(window, cx));
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));
        let item_info = cx.new(|cx| {
            let picker = ItemInfoState::new(window, cx);
            picker
        });
        let _subscriptions = vec![
            cx.subscribe(&label_picker, |_this, _, ev, _| match ev {
                LabelsPickerEvent::Selected(label) => {
                    println!("label picker selected: {:?}", label.clone());
                },
                LabelsPickerEvent::DeSelected(label) => {
                    println!("label picker deselected: {:?}", label.clone());
                },
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                    println!("Color changed to: {:?}", color);
                },
            }),
            cx.subscribe(&label_popover_list, |_this, _, ev: &LabelsPopoverEvent, _| match ev {
                LabelsPopoverEvent::Selected(label) => {
                    println!("label_popover_list select: {:?}", label);
                },
            }),
            cx.subscribe(&item_info, |this, _, _event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |_item_info, _cx| {
                    // item_info.handel_item_info_event(event, cx);
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
            company_list,
            selected_company: None,
            _subscriptions,
            item_info,
            popover_list,
            label_popover_list,
            label_picker,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_item() {
            self.selected_company = Some(company);
        }
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
            .child(
                section("label picker")
                    .child(LabelsPicker::new(&self.label_picker).cleanable(true)),
            )
            .child(section("popover_list").child(self.popover_list.clone()))
            .child(section("label popover list").child(self.label_popover_list.clone()))
            .child(ColorGroup::new(&self.color).large())
            .when_some(self.selected_color, |this, color| {
                this.child(
                    h_flex()
                        .gap_4()
                        .child(
                            div()
                                .id(SharedString::from(format!("color-{}", color.to_hex())))
                                .h_5()
                                .w_5()
                                .bg(color)
                                .border_1()
                                .border_color(color.darken(0.1)),
                        )
                        .child(color.to_hex()),
                )
            })
            .child(Divider::horizontal())
        // .child(
        //     List::new(&self.company_list)
        //         .p(px(8.))
        //         .flex_1()
        //         .w_full()
        //         .border_1()
        //         .border_color(cx.theme().border)
        //         .rounded(cx.theme().radius),
        // )
    }
}
