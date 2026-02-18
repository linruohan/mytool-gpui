use std::sync::Arc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::LabelModel;
use tracing::error;

use crate::todo_state::{DBState, LabelState, TodoStore};

// 刷新labels
async fn refresh_labels(cx: &mut AsyncApp, db: DatabaseConnection) {
    let labels = crate::state_service::load_labels(db).await;
    cx.update_global::<LabelState, _>(|state, _| {
        state.labels = labels.iter().map(|label| Arc::new(label.clone())).collect::<Vec<_>>();
    });
}
// 添加label
pub fn add_label(label: Arc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_label(label.clone(), db.clone()).await {
            Ok(new_label) => {
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_label(arc_label.clone());
                });
                // 同时刷新 LabelState 保持兼容性
                refresh_labels(cx, db.clone()).await;
            },
            Err(e) => error!("add_label failed: {:?}", e),
        }
    })
    .detach();
}
// 修改label
pub fn update_label(label: Arc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::mod_label(label.clone(), db.clone()).await {
            Ok(new_label) => {
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_label(arc_label.clone());
                });
                // 同时刷新 LabelState 保持兼容性
                refresh_labels(cx, db.clone()).await;
            },
            Err(e) => error!("update_label failed: {:?}", e),
        }
    })
    .detach();
}
// 删除label
pub fn delete_label(label: Arc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let label_id = label.id.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::del_label(label.clone(), db.clone()).await {
            Ok(_) => {
                // 增量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_label(&label_id);
                });
                // 同时刷新 LabelState 保持兼容性
                refresh_labels(cx, db.clone()).await;
            },
            Err(e) => error!("delete_label failed: {:?}", e),
        }
    })
    .detach();
}
