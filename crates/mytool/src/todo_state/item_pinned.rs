use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::get_items_pinned, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum PinnedItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct PinnedItemState {
    pub items: Vec<Rc<ItemModel>>,
}

impl Global for PinnedItemState {}

impl PinnedItemState {
    pub fn init(cx: &mut App) {
        let this = PinnedItemState { items: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = get_items_pinned(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state pinned_items: {}", list.len());
            cx.update_global::<PinnedItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新PinnedItemState
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let db = cx.global::<DBState>().conn.clone();
            cx.spawn(async move |cx| {
                let list = get_items_pinned(db.clone()).await;
                let rc_list: Vec<Rc<ItemModel>> =
                    list.iter().map(|pro| Rc::new(pro.clone())).collect();
                println!("state pinned_items updated: {}", list.len());
                cx.update_global::<PinnedItemState, _>(|state, _cx| {
                    state.items = rc_list;
                });
            })
            .detach();
        })
        .detach();
    }
}
