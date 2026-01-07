use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ProjectModel;

use crate::todo_state::{DBState, ProjectState};

// 刷新projects
#[allow(unused)]
async fn refresh_projects(cx: &mut AsyncApp, db: DatabaseConnection) {
    let projects = crate::service::load_projects(db).await;
    cx.update_global::<ProjectState, _>(|state, _| {
        state.projects =
            projects.iter().map(|project| Rc::new(project.clone())).collect::<Vec<_>>();
    })
    .ok();
}
// 添加project
#[allow(unused)]
pub fn add_project(project: Rc<ProjectModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::add_project(project.clone(), db.clone()).await.is_ok() {
            refresh_projects(cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改project
pub fn update_project(project: Rc<ProjectModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::mod_project(project.clone(), db.clone()).await.is_ok() {
            refresh_projects(cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除project
pub fn delete_project(project: Rc<ProjectModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_project(project.clone(), db.clone()).await {
            refresh_projects(cx, db.clone()).await;
        }
    })
    .detach();
}
