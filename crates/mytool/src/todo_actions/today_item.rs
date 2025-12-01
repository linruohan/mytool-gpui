use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;

use crate::{DBState, ItemState};

// 刷新items
async fn refresh_items(cx: &mut AsyncApp, db: DatabaseConnection) {
    let items = crate::service::get_items_today(db).await;
    cx.update_global::<ItemState, _>(|state, _| {
        state.set_items(items);
    })
    .ok();
}
// 添加item
pub fn add_today_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::add_item(item.clone(), db.clone()).await {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改item
pub fn update_today_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::mod_item(item.clone(), db.clone()).await {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除item
pub fn delete_today_item(item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_item(item.clone(), db.clone()).await {
            refresh_items(cx, db.clone()).await;
        }
    })
    .detach();
}
