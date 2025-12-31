use std::rc::Rc;

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
    pub items: Vec<Rc<ItemModel>>,
    active_item: Option<Rc<ItemModel>>,
}

impl Global for ScheduledItemState {}

impl ScheduledItemState {
    pub fn init(cx: &mut App) {
        let this = ScheduledItemState { items: vec![], active_item: None };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = get_items_scheduled(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("scheduled items: {}", list.len());
            let _ = cx.update_global::<ScheduledItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();
    }

    pub fn set_items(&mut self, items: impl IntoIterator<Item = ItemModel>) {
        self.items = items.into_iter().map(Rc::new).collect();
        self.active_item = None;
    }
}
