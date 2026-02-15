use std::sync::Arc;

use gpui::App;
use todos::entity::ProjectModel;
use tracing::error;

use crate::todo_state::{DBState, ProjectState, TodoStore};

// 添加project（使用增量更新，性能最优）
pub fn add_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_project(project.clone(), db.clone()).await {
            Ok(new_project) => {
                // 增量更新：只添加新项目到 TodoStore 和 ProjectState
                let arc_project = Arc::new(new_project);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_project(arc_project.clone());
                });
                let _ = cx.update_global::<ProjectState, _>(|state, _| {
                    state.projects.push(arc_project);
                });
            },
            Err(e) => error!("add_project failed: {:?}", e),
        }
    })
    .detach();
}

// 修改project（使用增量更新，性能最优）
pub fn update_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::mod_project(project.clone(), db.clone()).await {
            Ok(updated_project) => {
                // 增量更新：只更新修改的项目
                let arc_project = Arc::new(updated_project);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_project(arc_project.clone());
                });
                let _ = cx.update_global::<ProjectState, _>(|state, _| {
                    if let Some(pos) = state.projects.iter().position(|p| p.id == arc_project.id) {
                        state.projects[pos] = arc_project;
                    }
                });
            },
            Err(e) => error!("update_project failed: {:?}", e),
        }
    })
    .detach();
}

// 删除project（使用增量更新，性能最优）
pub fn delete_project(project: Arc<ProjectModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::del_project(project.clone(), db.clone()).await {
            Ok(_) => {
                // 增量更新：只删除指定的项目
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_project(&project_id);
                });
                let _ = cx.update_global::<ProjectState, _>(|state, _| {
                    state.projects.retain(|p| p.id != project_id);
                });
            },
            Err(e) => error!("delete_project failed: {:?}", e),
        }
    })
    .detach();
}
