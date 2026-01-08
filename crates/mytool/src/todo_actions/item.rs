use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;

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
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::add_item(item.clone(), db.clone()).await.is_ok() {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改item
pub fn update_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::mod_item(item.clone(), db.clone()).await.is_ok() {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除item
pub fn delete_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_item(item.clone(), db.clone()).await {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
pub fn completed_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::finish_item(item.clone(), true, false, db.clone()).await.is_ok() {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
pub fn uncompleted_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) =
            crate::service::finish_item(item.clone(), false, false, db.clone()).await
        {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}

pub fn set_item_pinned(item: Rc<ItemModel>, pinned: bool, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::pin_item(item.clone(), pinned, db.clone()).await {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
