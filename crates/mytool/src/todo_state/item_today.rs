use std::sync::Arc;

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
    pub items: Vec<Arc<ItemModel>>,
}

impl Global for TodayItemState {}

impl TodayItemState {
    pub fn init(cx: &mut App) {
        let this = TodayItemState { items: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            // 只获取今天到期的任务
            let today_items = get_items_today(db.clone()).await;

            let arc_list: Vec<Arc<ItemModel>> =
                today_items.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state today_items: {}", today_items.len());
            for item in &today_items {
                println!(
                    "  today item: {} - due: {:?}, checked: {}, pinned: {}",
                    item.content, item.due, item.checked, item.pinned
                );
            }
            cx.update_global::<TodayItemState, _>(|state, _cx| {
                state.items = arc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新TodayItemState
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let db = cx.global::<DBState>().conn.clone();
            cx.spawn(async move |cx| {
                // 只获取今天到期的任务
                let today_items = get_items_today(db.clone()).await;

                let arc_list: Vec<Arc<ItemModel>> =
                    today_items.iter().map(|pro| Arc::new(pro.clone())).collect();
                println!("state today_items updated: {}", today_items.len());
                for item in &today_items {
                    println!(
                        "  updated today item: {} - due: {:?}, checked: {}, pinned: {}",
                        item.content, item.due, item.checked, item.pinned
                    );
                }
                cx.update_global::<TodayItemState, _>(|state, _cx| {
                    state.items = arc_list;
                });
            })
            .detach();
        })
        .detach();
    }
}
