use std::sync::Arc;

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
    pub items: Vec<Arc<ItemModel>>,
}

impl Global for InboxItemState {}

impl InboxItemState {
    pub fn init(cx: &mut App) {
        let this = InboxItemState { items: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = get_inbox_items(db.clone()).await;
            let arc_list: Vec<Arc<ItemModel>> =
                list.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state inbox_items: {}", list.len());
            cx.update_global::<InboxItemState, _>(|state, _cx| {
                state.items = arc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新InboxItemState
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let db = cx.global::<DBState>().conn.clone();
            cx.spawn(async move |cx| {
                let list = get_inbox_items(db.clone()).await;
                let arc_list: Vec<Arc<ItemModel>> =
                    list.iter().map(|pro| Arc::new(pro.clone())).collect();
                println!("state inbox_items updated: {}", list.len());
                cx.update_global::<InboxItemState, _>(|state, _cx| {
                    state.items = arc_list;
                });
            })
            .detach();
        })
        .detach();
    }
}
