use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::LabelModel;
use tracing::error;

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
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::add_label(label.clone(), db.clone()).await {
        Ok(_) => refresh_labels(cx, db.clone()).await,
        Err(e) => error!("add_label failed: {:?}", e),
    })
    .detach();
}
// 修改label
pub fn update_label(label: Rc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::mod_label(label.clone(), db.clone()).await {
        Ok(_) => refresh_labels(cx, db.clone()).await,
        Err(e) => error!("update_label failed: {:?}", e),
    })
    .detach();
}
// 删除label
pub fn delete_label(label: Rc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| match crate::service::del_label(label.clone(), db.clone()).await {
        Ok(_) => refresh_labels(cx, db.clone()).await,
        Err(e) => error!("delete_label failed: {:?}", e),
    })
    .detach();
}
