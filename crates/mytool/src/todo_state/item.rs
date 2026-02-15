use std::sync::Arc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{state_service::load_items, todo_state::DBState};

pub struct ItemState {
    pub items: Vec<Arc<ItemModel>>,
}

impl Global for ItemState {}

impl ItemState {
    pub fn init(cx: &mut App) {
        let this = ItemState { items: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = load_items(db.clone()).await;
            let arc_list: Vec<Arc<ItemModel>> =
                list.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state items: {}", list.len());
            cx.update_global::<ItemState, _>(|state, _cx| {
                state.items = arc_list;
            });
        })
        .detach();
    }
}
