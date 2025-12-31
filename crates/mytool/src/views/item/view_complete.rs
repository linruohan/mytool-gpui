use std::rc::Rc;

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

use crate::{
    ItemListDelegate,
    todo_actions::{completed_item, uncompleted_item},
    todo_state::CompleteItemState,
};

pub enum ItemCompletedEvent {
    UnFinished(Rc<ItemModel>),
    Finished(Rc<ItemModel>),
}

impl EventEmitter<ItemCompletedEvent> for ItemsCompletedPanel {}
pub struct ItemsCompletedPanel {
    input_esc: Entity<InputState>,
    pub item_list: Entity<ListState<ItemListDelegate>>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ItemsCompletedPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter DB URL").clean_on_escape());

        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).selectable(true));
        let item_list_clone = item_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<CompleteItemState>(move |_this, cx| {
                let items = cx.global::<CompleteItemState>().items.clone();
                let _ = cx.update_entity(&item_list_clone, |list, cx| {
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

    fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
        self.item_list
            .read(cx)
            .delegate()
            .matched_items
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }

    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn handle_complete_event(&mut self, event: &ItemCompletedEvent, cx: &mut Context<Self>) {
        match event {
            ItemCompletedEvent::UnFinished(item) => {
                uncompleted_item(item.clone(), cx);
            },
            ItemCompletedEvent::Finished(item) => {
                completed_item(item.clone(), cx);
            },
        }
    }

    pub fn show_unfinish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), &cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to unfinish the item?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window, cx| {
                                let view = view.clone();
                                let mut item = item.clone();
                                let item_mut = Rc::make_mut(&mut item);
                                item_mut.checked = false; //切换为未完成状态
                                view.update(cx, |_view, cx| {
                                    cx.emit(ItemCompletedEvent::UnFinished(item.clone()));
                                    cx.notify();
                                });
                                window.push_notification("You have unfinished item ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window, cx| {
                            window.push_notification("You have canceled.", cx);
                            true
                        })
                });
            };
        }
    }
}

impl Render for ItemsCompletedPanel {
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
