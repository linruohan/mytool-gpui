use std::{collections::HashMap, rc::Rc};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, actions, div, prelude::FluentBuilder,
};
use gpui_component::{
    IconName, Sizable,
    button::Button,
    collapsible::Collapsible,
    divider::Divider,
    gray_300,
    group_box::{GroupBox, GroupBoxVariants},
    h_flex,
    list::{ListEvent, ListState},
    tag::Tag,
    v_flex,
};
use todos::entity::ItemModel;

use crate::{
    ItemInfo, ItemInfoState, ItemListDelegate, ItemRow, ItemRowEvent, ItemRowState,
    LabelsPopoverEvent, LabelsPopoverList,
    popover_list::PopoverList,
    section,
    service::load_items,
    todo_state::{DBState, ItemState},
};

actions!(list_story, [SelectedCompany]);
pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Rc<ItemModel>>,
    _subscriptions: Vec<Subscription>,
    item_infos: HashMap<String, Entity<ItemInfoState>>,
    pub popover_list: Entity<PopoverList>,
    pub label_popover_list: Entity<LabelsPopoverList>,
    item_row: Entity<ItemRowState>,
    item_open: Option<Rc<ItemModel>>,
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
        let item = Rc::new(ItemModel {
            id: "1".to_string(),
            content: "Item 1".to_string(),
            description: Some("This is item 1".to_string()),
            ..Default::default()
        });
        let item_row = cx.new(|cx| ItemRowState::new(item.clone(), window, cx));
        let popover_list = cx.new(|cx| PopoverList::new(window, cx));
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));
        let item_infos = {
            let items = cx.global::<ItemState>().items.clone();
            items
                .iter()
                .map(|item| {
                    let entity = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
                    (item.id.clone(), entity)
                })
                .collect()
        };
        let _subscriptions = vec![
            cx.observe_global_in::<ItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<ItemState>().items.clone();

                // 将state_items转换为HashMap便于快速查找
                let items_by_id: HashMap<String, Rc<ItemModel>> =
                    state_items.iter().map(|item| (item.id.clone(), item.clone())).collect();

                // 更新或删除现有的item_infos
                this.item_infos.retain(|item_id, entity| {
                    if let Some(updated_item) = items_by_id.get(item_id) {
                        // 更新
                        cx.update_entity(entity, |item_info, _cx| {
                            item_info.item = updated_item.clone();
                        });
                        true
                    } else {
                        // 不存在，删除
                        // cx.remove_entity(entity);
                        false
                    }
                });

                // 添加新的items（那些不在item_infos中的）
                for (item_id, item) in items_by_id {
                    if !this.item_infos.contains_key(&item_id) {
                        let entity = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
                        this.item_infos.insert(item_id, entity);
                    }
                }
                cx.notify();
            }),
            cx.subscribe(&item_row, |_this, _, ev, _| {
                if let ItemRowEvent::Added(label) = ev {
                    println!("label picker selected: {:?}", label.clone());
                }
            }),
            cx.subscribe(&label_popover_list, |_this, _, ev: &LabelsPopoverEvent, _| match ev {
                LabelsPopoverEvent::Selected(label) => {
                    println!("label_popover_list select: {:?}", label);
                },
                LabelsPopoverEvent::DeSelected(_model) => {},
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
            focus_handle: cx.focus_handle(),
            company_list,
            selected_company: None,
            _subscriptions,
            item_infos,
            popover_list,
            label_popover_list,
            item_row,
            item_open: None,
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
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for ListStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _item = Rc::new(ItemModel {
            id: "1".to_string(),
            content: "Item 1".to_string(),
            description: Some("This is item 1".to_string()),
            ..Default::default()
        });

        let _items = [
            Rc::new(ItemModel {
                id: "1".to_string(),
                content: "Item 1".to_string(),
                description: Some("This is item 1".to_string()),
                ..Default::default()
            }),
            Rc::new(ItemModel {
                id: "2".to_string(),
                content: "Item 2".to_string(),
                description: Some("This is item 2".to_string()),
                ..Default::default()
            }),
            Rc::new(ItemModel {
                id: "3".to_string(),
                content: "Item 3".to_string(),
                description: Some("This is item 3".to_string()),
                ..Default::default()
            }),
        ];
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .w_full()
            .gap_4()
            // .child(section("item_info").child(ItemInfo::new(&self.item_info)))
            .child(section("item row").child(ItemRow::new(&self.item_row)))
            .child(section("popover_list").child(self.popover_list.clone()))
            .child(section("label popover list").child(self.label_popover_list.clone()))
            .child(Divider::horizontal())
            .child(
                section("Card").child(v_flex()
                    .children(
                        self.item_infos.clone().into_values().map(|item_info_state| {
                            let item = item_info_state.read(cx).item.clone();
                            let is_open = match &self.item_open {
                                Some(open_item) => open_item.id == item.id,
                                None => false,
                            };
                            GroupBox::new().outline().w_full()
                                .child(
                                    div().id(format!("item-{}", item.id.clone()))
                                         .child(
                                             Collapsible::new()
                                                 .gap_1()
                                                 .open(is_open)
                                                 .child(
                                                     h_flex().child(
                                                         v_flex()
                                                             .child(item.content.clone())
                                                             .child(div().text_color(gray_300()).child(item.description.clone().unwrap_or_default())))
                                                             .child(Tag::info().child("+1.5%").outline().rounded_full().small())
                                                             .child(
                                                                 Button::new("toggle2")
                                                                     .small()
                                                                     .outline()
                                                                     .icon(IconName::ChevronDown)
                                                                     .label("Details")
                                                                     .when(is_open, |this| {
                                                                         this.icon(IconName::ChevronUp)
                                                                     })
                                                                     .on_click({
                                                                         cx.listener(move |this, _, _, cx| {
                                                                             if is_open { this.item_open = None } else {
                                                                                 this.item_open = Some(item.clone());
                                                                             }
                                                                             cx.notify();
                                                                         })
                                                                     }),
                                                             ),
                                                 )
                                                 .content(v_flex().gap_2().child(ItemInfo::new(&item_info_state)))
                                         )
                                )
                        }))))
    }
}
