use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    input::InputState,
    list::{List, ListEvent, ListState},
};
use todos::entity::ItemModel;

use crate::{DBState, ItemListDelegate, get_items_completed, load_items};
pub enum ItemCompletedEvent {
    UnFinished(Rc<ItemModel>),
}

impl EventEmitter<ItemCompletedEvent> for ItemsCompletedPanel {}
pub struct ItemsCompletedPanel {
    input_esc: Entity<InputState>,
    pub item_list: Entity<ListState<ItemListDelegate>>,
    item_due: Option<String>,
    is_loading: bool,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ItemsCompletedPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter DB URL").clean_on_escape());

        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).selectable(true));

        let _subscriptions =
            vec![cx.subscribe_in(&item_list, window, |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_item(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().content.clone(), window, cx);
                        cx.notify();
                    })
                }
            })];

        let item_list_clone = item_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let items = get_items_completed(db.clone()).await;
            let rc_items: Vec<Rc<ItemModel>> =
                items.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("completed items: {}", items.len());
            let _ = cx
                .update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            input_esc,
            item_due: None,
            is_loading: false,
            item_list,
            active_index: Some(0),
            _subscriptions,
        }
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

    pub fn handle_item_event(&mut self, event: &ItemCompletedEvent, cx: &mut Context<Self>) {
        match event {
            ItemCompletedEvent::UnFinished(item) => {
                println!("toggle unfinished item:");
                self.unfinish_item(cx, item.clone())
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

    // 更新items
    fn get_items(&mut self, cx: &mut Context<Self>) {
        if !self.is_loading {
            return;
        }
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this, cx| {
            let db = db.lock().await;
            let items = load_items(db.clone()).await;
            let rc_items: Vec<Rc<ItemModel>> =
                items.iter().map(|pro| Rc::new(pro.clone())).collect();

            this.update(cx, |this, cx| {
                this.item_list.update(cx, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                });

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn unfinish_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ItemsCompletedPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::mod_item(item.clone(), db.clone()).await;
            println!("add_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_items(cx);
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
