use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::{ItemModel, ProjectModel};

use crate::todo_state::{DBState, ProjectItemState};

// 刷新items
async fn refresh_project_items(project_id: &str, cx: &mut AsyncApp, db: DatabaseConnection) {
    let items = crate::service::get_items_by_project_id(project_id, db).await;
    cx.update_global::<ProjectItemState, _>(|state, _| {
        state.set_items(items);
    })
    .ok();
}
// 添加item
pub fn add_project_item(project: Rc<ProjectModel>, item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::add_item(item.clone(), db.clone()).await {
            refresh_project_items(&project.id.clone(), cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改item
pub fn update_project_item(project: Rc<ProjectModel>, item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_) = crate::service::mod_item(item.clone(), db.clone()).await {
            refresh_project_items(&project.id.clone(), cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除item
pub fn delete_project_item(project: Rc<ProjectModel>, item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_item(item.clone(), db.clone()).await {
            refresh_project_items(&project.id.clone(), cx, db.clone()).await;
        }
    })
    .detach();
}
