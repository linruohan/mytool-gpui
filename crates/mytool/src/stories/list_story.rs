use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, actions,
};
use gpui_component::{
    IndexPath,
    checkbox::Checkbox,
    divider::Divider,
    h_flex,
    list::{ListEvent, ListState},
    select::{SearchableVec, Select, SelectEvent, SelectGroup, SelectItem, SelectState},
    v_flex,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use todos::entity::{ItemModel, LabelModel};

use crate::{
    ItemInfo, ItemInfoEvent, ItemInfoState, ItemListDelegate, ItemRow, ItemRowEvent, ItemRowState,
    LabelsPopoverEvent, LabelsPopoverList,
    popover_list::PopoverList,
    section,
    service::load_items,
    todo_state::{DBState, LabelState},
};

actions!(list_story, [SelectedCompany]);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LabelSelect {
    label: Rc<LabelModel>,
    selected: bool,
    pub checked: bool,
}
impl LabelSelect {
    fn new(label: Rc<LabelModel>, checked: bool) -> Self {
        Self { label, selected: false, checked }
    }

    fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    fn name(&self) -> String {
        self.label.name.clone()
    }
}
impl SelectItem for LabelSelect {
    type Value = Rc<LabelModel>;

    fn title(&self) -> SharedString {
        self.label.name.clone().into()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(format!("{} ", self.label.name.clone()).into_any_element())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex().child(Checkbox::new("is").checked(self.checked)).child(self.label.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.label
    }
}
pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Rc<ItemModel>>,
    _subscriptions: Vec<Subscription>,
    item_info: Entity<ItemInfoState>,
    pub popover_list: Entity<PopoverList>,
    pub label_popover_list: Entity<LabelsPopoverList>,
    item_row: Entity<ItemRowState>,
    label_list_select: Entity<SelectState<SearchableVec<SelectGroup<LabelSelect>>>>,
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
        let item_info = cx.new(|cx| {
            let picker = ItemInfoState::new(window, cx);
            picker
        });
        let label_list = cx.global::<LabelState>().labels.clone();
        let mut grouped_countries: SearchableVec<SelectGroup<LabelSelect>> =
            SearchableVec::new(vec![]);
        for (prefix, items) in label_list.iter().chunk_by(|c| c.name.clone()).into_iter() {
            let items = items
                .into_iter()
                .map(|item| LabelSelect::new(item.clone(), false))
                .collect::<Vec<LabelSelect>>();
            grouped_countries.push(SelectGroup::new(prefix.to_string()).items(items));
        }

        let label_list_select = cx.new(|cx| {
            SelectState::new(grouped_countries, Some(IndexPath::default()), window, cx)
                .searchable(true)
        });
        let _subscriptions = vec![
            cx.subscribe_in(&label_list_select, window, Self::on_select_event),
            cx.subscribe(&item_row, |_this, _, ev, _| match ev {
                ItemRowEvent::Added(label) => {
                    println!("label picker selected: {:?}", label.clone());
                },
                _ => {},
            }),
            cx.subscribe(&label_popover_list, |_this, _, ev: &LabelsPopoverEvent, _| match ev {
                LabelsPopoverEvent::Selected(label) => {
                    println!("label_popover_list select: {:?}", label);
                },
                LabelsPopoverEvent::DeSelected(_model) => {},
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
            focus_handle: cx.focus_handle(),
            company_list,
            selected_company: None,
            _subscriptions,
            item_info,
            popover_list,
            label_popover_list,
            item_row,
            label_list_select,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_item() {
            self.selected_company = Some(company);
        }
    }

    fn on_select_event(
        &mut self,
        _: &Entity<SelectState<SearchableVec<SelectGroup<LabelSelect>>>>,
        event: &SelectEvent<SearchableVec<SelectGroup<LabelSelect>>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            SelectEvent::Confirm(value) => println!("Selected country: {:?}", value),
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
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(section("item_info").child(ItemInfo::new(&self.item_info)))
            .child(section("item row").child(ItemRow::new(&self.item_row)))
            .child(section("Select").child(Select::new(&self.label_list_select).cleanable(true)))
            .child(section("popover_list").child(self.popover_list.clone()))
            .child(section("label popover list").child(self.label_popover_list.clone()))
            .child(Divider::horizontal())
    }
}
