use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ItemModel;

use crate::{service::get_items_today, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum TodayItemStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct TodayItemState {
    pub items: Vec<Rc<ItemModel>>,
}

impl Global for TodayItemState {}

impl TodayItemState {
    pub fn init(cx: &mut App) {
        let this = TodayItemState { items: vec![] };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = get_items_today(db.clone()).await;
            let rc_list: Vec<Rc<ItemModel>> = list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state today_items: {}", list.len());
            cx.update_global::<TodayItemState, _>(|state, _cx| {
                state.items = rc_list;
            });
        })
        .detach();
    }
}
