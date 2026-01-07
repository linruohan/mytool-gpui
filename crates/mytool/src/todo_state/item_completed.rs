use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::get_items_completed, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum CompleteItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct CompleteItemState {
    pub items: Vec<Rc<ItemModel>>,
    active_item: Option<Rc<ItemModel>>,
}

impl Global for CompleteItemState {}

impl CompleteItemState {
    pub fn init(cx: &mut App) {
        let this = CompleteItemState { items: vec![], active_item: None };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = get_items_completed(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state completed_items: {}", list.len());
            let _ = cx.update_global::<CompleteItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();
    }
}
