use std::sync::Arc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ProjectModel;
use tracing::error;

use crate::todo_state::{DBState, ProjectState};

// 刷新projects
#[allow(unused)]
async fn refresh_projects(cx: &mut AsyncApp, db: DatabaseConnection) {
    let projects = crate::service::load_projects(db).await;
    cx.update_global::<ProjectState, _>(|state, _| {
        state.projects =
            projects.iter().map(|project| Arc::new(project.clone())).collect::<Vec<_>>();
    });
}
// 添加project
#[allow(unused)]
pub fn add_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::add_project(project.clone(), db.clone()).await {
            Ok(_) => refresh_projects(cx, db.clone()).await,
            Err(e) => error!("add_project failed: {:?}", e),
        }
    })
    .detach();
}
// 修改project
#[allow(unused)]
pub fn update_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::mod_project(project.clone(), db.clone()).await {
            Ok(_) => refresh_projects(cx, db.clone()).await,
            Err(e) => error!("update_project failed: {:?}", e),
        }
    })
    .detach();
}
// 删除project
#[allow(unused)]
pub fn delete_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::del_project(project.clone(), db.clone()).await {
            Ok(_store) => refresh_projects(cx, db.clone()).await,
            Err(e) => error!("delete_project failed: {:?}", e),
        }
    })
    .detach();
}
