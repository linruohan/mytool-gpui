use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;
use tracing::error;

use crate::todo_state::{DBState, ItemState};

// 刷新items
async fn refresh_items(cx: &mut AsyncApp, db: DatabaseConnection) {
    let items = crate::service::load_items(db).await;
    let rc_items = items.iter().map(|item| Rc::new(item.clone())).collect::<Vec<_>>();
    cx.update_global::<ItemState, _>(|state, _| {
        state.items = rc_items.clone();
    });
}
// 添加item
pub fn add_item(item: Rc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::add_item(item.clone(), db.clone()).await {
        Ok(_) => {
            refresh_items(cx, db.clone()).await;
        },
        Err(e) => {
            error!("add_item failed: {:?}", e);
        },
    })
    .detach();
}
// 修改item
pub fn update_item(item: Rc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::mod_item(item.clone(), db.clone()).await {
        Ok(_) => {
            refresh_items(cx, db.clone()).await;
        },
        Err(e) => {
            error!("update_item failed: {:?}", e);
        },
    })
    .detach();
}
// 删除item
pub fn delete_item(item: Rc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::del_item(item.clone(), db.clone()).await {
        Ok(_store) => {
            refresh_items(cx, db.clone()).await;
        },
        Err(e) => {
            error!("delete_item failed: {:?}", e);
        },
    })
    .detach();
}
pub fn completed_item(item: Rc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::finish_item(item.clone(), true, false, db.clone()).await {
            Ok(_) => {
                refresh_items(cx, db.clone()).await;
            },
            Err(e) => {
                error!("completed_item failed: {:?}", e);
            },
        }
    })
    .detach();
}
pub fn uncompleted_item(item: Rc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::finish_item(item.clone(), false, false, db.clone()).await {
            Ok(_store) => {
                refresh_items(cx, db.clone()).await;
            },
            Err(e) => {
                error!("uncompleted_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

pub fn set_item_pinned(item: Rc<ItemModel>, pinned: bool, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::pin_item(item.clone(), pinned, db.clone()).await {
            Ok(_store) => {
                refresh_items(cx, db.clone()).await;
            },
            Err(e) => {
                error!("set_item_pinned failed: {:?}", e);
            },
        }
    })
    .detach();
}
