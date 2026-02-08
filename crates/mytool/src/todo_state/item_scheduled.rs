use std::sync::Arc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::get_items_scheduled, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum ScheduledItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct ScheduledItemState {
    pub items: Vec<Arc<ItemModel>>,
}

impl Global for ScheduledItemState {}

impl ScheduledItemState {
    pub fn init(cx: &mut App) {
        let this = ScheduledItemState { items: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = get_items_scheduled(db.clone()).await;
            let arc_list: Vec<Arc<ItemModel>> =
                list.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state scheduled_items: {}", list.len());
            cx.update_global::<ScheduledItemState, _>(|state, _cx| {
                state.items = arc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新ScheduledItemState
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let db = cx.global::<DBState>().conn.clone();
            cx.spawn(async move |cx| {
                let list = get_items_scheduled(db.clone()).await;
                let arc_list: Vec<Arc<ItemModel>> =
                    list.iter().map(|pro| Arc::new(pro.clone())).collect();
                println!("state scheduled_items updated: {}", list.len());
                cx.update_global::<ScheduledItemState, _>(|state, _cx| {
                    state.items = arc_list;
                });
            })
            .detach();
        })
        .detach();
    }
}
