use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::LabelModel;

use crate::{DBState, LabelState};

// 刷新labels
async fn refresh_labels(cx: &mut AsyncApp, db: DatabaseConnection) {
    let labels = crate::service::load_labels(db).await;
    cx.update_global::<LabelState, _>(|state, _| {
        state.set_labels(labels);
    })
      .ok();
}
// 添加label
pub fn add_label(label: Rc<LabelModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::add_label(label.clone(), db.clone()).await {
            refresh_labels(cx, db.clone()).await;
        }
    })
      .detach();
}
// 修改label
pub fn update_label(label: Rc<LabelModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::mod_label(label.clone(), db.clone()).await {
            refresh_labels(cx, db.clone()).await;
        }
    })
      .detach();
}
// 删除label
pub fn delete_label(label: Rc<LabelModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_label(label.clone(), db.clone()).await {
            refresh_labels(cx, db.clone()).await;
        }
    })
      .detach();
}
