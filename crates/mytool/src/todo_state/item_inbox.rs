use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::get_inbox_items, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum InboxItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct InboxItemState {
    pub items: Vec<Rc<ItemModel>>,
    active_item: Option<Rc<ItemModel>>,
}

impl Global for InboxItemState {}

impl InboxItemState {
    pub fn init(cx: &mut App) {
        let this = InboxItemState { items: vec![], active_item: None };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = get_inbox_items(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state inbox_items: {}", list.len());
            let _ = cx.update_global::<InboxItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新InboxItemState
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let conn = cx.global::<DBState>().conn.clone();
            cx.spawn(async move |cx| {
                let db = conn.lock().await;
                let list = get_inbox_items(db.clone()).await;
                let rc_list: Vec<Rc<ItemModel>> =
                    list.iter().map(|pro| Rc::new(pro.clone())).collect();
                println!("state inbox_items updated: {}", list.len());
                let _ = cx.update_global::<InboxItemState, _>(|state, _cx| {
                    state.items = rc_list;
                });
            })
            .detach();
        })
        .detach();
    }
}
