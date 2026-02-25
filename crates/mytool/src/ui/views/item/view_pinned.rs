use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    input::InputState,
    list::{List, ListEvent, ListState},
};
use todos::entity::ItemModel;

use crate::{ItemListDelegate, todo_actions::set_item_pinned_optimistic, todo_state::TodoStore};

pub enum ItemsPinnedEvent {
    Pinned(Arc<ItemModel>),
    UnPinned(Arc<ItemModel>),
}

impl EventEmitter<ItemsPinnedEvent> for ItemsPinnedPanel {}
pub struct ItemsPinnedPanel {
    input_esc: Entity<InputState>,
    pub item_list: Entity<ListState<ItemListDelegate>>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ItemsPinnedPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter DB URL").clean_on_escape());

        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).selectable(true));
        let item_list_clone = item_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<TodoStore>(move |_this, cx| {
                let items = cx.global::<TodoStore>().pinned_items();
                cx.update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(items);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe_in(&item_list, window, |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_item(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().content.clone(), window, cx);
                        cx.notify();
                    })
                }
            }),
        ];

        Self { input_esc, item_list, active_index: Some(0), _subscriptions }
    }

    fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Arc<ItemModel>> {
        self.item_list
            .read(cx)
            .delegate()
            .matched_items
            .get(ix.section)
            .and_then(|c: &Vec<Arc<ItemModel>>| c.get(ix.row))
            .cloned()
    }

    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn handle_pin_event(&mut self, event: &ItemsPinnedEvent, cx: &mut Context<Self>) {
        match event {
            ItemsPinnedEvent::Pinned(item) => {
                set_item_pinned_optimistic(item.clone(), true, cx);
            },
            ItemsPinnedEvent::UnPinned(item) => {
                set_item_pinned_optimistic(item.clone(), false, cx);
            },
        }
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to pinned the item?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
                                let view = view.clone();
                                let mut item_model = (*item).clone();
                                item_model.pinned = true;
                                let updated_item = Arc::new(item_model);
                                view.update(cx, |_view, cx| {
                                    cx.emit(ItemsPinnedEvent::Pinned(updated_item.clone()));
                                    cx.notify();
                                });
                                window.push_notification("You have pinned item ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window: &mut Window, cx| {
                            window.push_notification("You have canceled.", cx);
                            true
                        })
                });
            };
        }
    }
}

impl Render for ItemsPinnedPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        List::new(&self.item_list)
            .p(px(2.))
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
