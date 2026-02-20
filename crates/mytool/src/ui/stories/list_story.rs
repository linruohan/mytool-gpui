use std::{collections::HashMap, sync::Arc};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, actions,
};
use gpui_component::{
    divider::Divider,
    list::{ListEvent, ListState},
    v_flex,
};
use todos::entity::ItemModel;

use crate::{
    ItemListDelegate, ItemRow, ItemRowState, LabelsPopoverEvent, LabelsPopoverList,
    popover_list::PopoverList,
    section,
    state_service::load_items,
    todo_state::{DBState, TodoStore},
};

actions!(list_story, [SelectedCompany]);
pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<ItemListDelegate>>,
    selected_company: Option<Arc<ItemModel>>,
    _subscriptions: Vec<Subscription>,
    item_rows: HashMap<String, Entity<ItemRowState>>,
    pub popover_list: Entity<PopoverList>,
    pub label_popover_list: Entity<LabelsPopoverList>,
    cached_version: usize,
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
        let popover_list = cx.new(|cx| PopoverList::new(window, cx));
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));
        let item_rows = {
            let items = cx.global::<TodoStore>().all_items.clone();
            items
                .iter()
                .map(|item| {
                    let entity = cx.new(|cx| ItemRowState::new(item.clone(), window, cx));
                    (item.id.clone(), entity)
                })
                .collect()
        };
        let _subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();
                let current_version = store.version();

                // 版本号未变化，跳过更新
                if this.cached_version == current_version {
                    return;
                }

                this.cached_version = current_version;
                let state_items = store.all_items.clone();

                // 将state_items转换为HashMap便于快速查找
                let items_by_id: HashMap<String, Arc<ItemModel>> =
                    state_items.iter().map(|item| (item.id.clone(), item.clone())).collect();

                // 更新或删除现有的item_infos
                this.item_rows.retain(|item_id, entity| {
                    if let Some(updated_item) = items_by_id.get(item_id) {
                        // 更新
                        cx.update_entity(entity, |item_info, _cx| {
                            item_info.item = updated_item.clone();
                        });
                        true
                    } else {
                        // 不存在，删除
                        false
                    }
                });

                // 添加新的items（那些不在item_infos中的）
                for (item_id, item) in items_by_id {
                    if !this.item_rows.contains_key(&item_id) {
                        let entity = cx.new(|cx| ItemRowState::new(item.clone(), window, cx));
                        this.item_rows.insert(item_id, entity);
                    }
                }
                cx.notify();
            }),
            cx.subscribe(&label_popover_list, |_this, _, ev: &LabelsPopoverEvent, _| match ev {
                LabelsPopoverEvent::Selected(label) => {
                    println!("label_popover_list select: {:?}", label);
                },
                LabelsPopoverEvent::DeSelected(_model) => {},
                LabelsPopoverEvent::LabelsChanged(_labels) => {},
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
            let labels = load_items((*db).clone()).await;
            let rc_labels: Vec<Arc<ItemModel>> =
                labels.iter().map(|label| Arc::new(label.clone())).collect();
            println!("list_story: len labels: {}", rc_labels.len());
            cx.update_entity(&company_list_clone, |list, cx| {
                list.delegate_mut().update_items(rc_labels);
                cx.notify();
            });
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            company_list,
            selected_company: None,
            _subscriptions,
            item_rows,
            popover_list,
            label_popover_list,
            cached_version: 0,
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
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .w_full()
            .gap_4()
            .child(section("popover_list").child(self.popover_list.clone()))
            .child(section("label popover list").child(self.label_popover_list.clone()))
            .child(Divider::horizontal())
            .child(v_flex().children(
                self.item_rows.clone().into_values().map(|item| ItemRow::new(&item.clone())),
            ))
    }
}
