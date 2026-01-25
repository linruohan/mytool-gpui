use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::load_items, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum ItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct ItemState {
    pub items: Vec<Rc<ItemModel>>,
}

impl Global for ItemState {}

impl ItemState {
    pub fn init(cx: &mut App) {
        let this = ItemState { items: vec![] };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_items(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state items: {}", list.len());
            cx.update_global::<ItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();
    }
}
