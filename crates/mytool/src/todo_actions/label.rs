use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::LabelModel;

use crate::todo_state::{DBState, LabelState};

// 刷新labels
async fn refresh_labels(cx: &mut AsyncApp, db: DatabaseConnection) {
    let labels = crate::service::load_labels(db).await;
    cx.update_global::<LabelState, _>(|state, _| {
        state.labels = labels.iter().map(|label| Rc::new(label.clone())).collect::<Vec<_>>();
    });
}
// 添加label
pub fn add_label(label: Rc<LabelModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::add_label(label.clone(), db.clone()).await.is_ok() {
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
        if crate::service::mod_label(label.clone(), db.clone()).await.is_ok() {
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
        if crate::service::del_label(label.clone(), db.clone()).await.is_ok() {
            refresh_labels(cx, db.clone()).await;
        }
    })
    .detach();
}
