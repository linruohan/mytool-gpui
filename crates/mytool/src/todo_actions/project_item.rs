use std::rc::Rc;

use gpui::{App, AsyncApp, BorrowAppContext};
use sea_orm::DatabaseConnection;
use todos::entity::{ItemModel, ProjectModel};

use crate::todo_state::{DBState, ProjectState};

pub fn load_project_items(project: Rc<ProjectModel>, cx: &mut App) {
    // 记录当前激活的 project，供异步刷新时做竞态保护
    cx.update_global::<ProjectState, _>(|state, _| {
        state.active_project = Some(project.clone());
    });

    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        refresh_project_items(&project.id.clone(), cx, db.clone()).await
    })
    .detach();
}
// 刷新items
async fn refresh_project_items(project_id: &str, cx: &mut AsyncApp, db: DatabaseConnection) {
    let items = crate::service::get_items_by_project_id(project_id, db).await;
    let rc_items = items.iter().map(|item| Rc::new(item.clone())).collect::<Vec<_>>();
    println!("project items: {}", rc_items.len());
    // 只在当前激活项目仍然是该 project_id 时更新，避免快速切换导致旧请求覆盖新项目的 items
    cx.update_global::<ProjectState, _>(|state, _| {
        if let Some(active) = &state.active_project
            && active.id == project_id
        {
            state.items = rc_items.clone();
        }
    });
}
// 添加item
pub fn add_project_item(project: Rc<ProjectModel>, item: Rc<ItemModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::add_item(item.clone(), db.clone()).await.is_ok() {
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
        if crate::service::mod_item(item.clone(), db.clone()).await.is_ok() {
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
