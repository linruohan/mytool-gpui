use std::sync::Arc;

use gpui::App;
use todos::entity::ProjectModel;
use tracing::{error, info};

use crate::{
    error_handler::{AppError, ErrorHandler, validation},
    todo_state::{DBState, TodoStore},
};

// 添加project（使用增量更新，性能最优）
pub fn add_project(project: Arc<ProjectModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_project_name(&project.name) {
        let context = ErrorHandler::handle_with_location(e, "add_project");
        error!("{}", context.format_user_message());
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_project(project.clone(), db.clone()).await {
            Ok(new_project) => {
                info!("Successfully added project: {}", new_project.id);
                // 增量更新：只添加新项目到 TodoStore
                let arc_project = Arc::new(new_project);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_project(arc_project);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_project",
                    &project.id,
                );
                error!("{}", context.format_user_message());
            }
        }
    })
    .detach();
}

// 修改project（使用增量更新，性能最优）
pub fn update_project(project: Arc<ProjectModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_project_name(&project.name) {
        let context = ErrorHandler::handle_with_location(e, "update_project");
        error!("{}", context.format_user_message());
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::mod_project(project.clone(), db.clone()).await {
            Ok(updated_project) => {
                info!("Successfully updated project: {}", updated_project.id);
                // 增量更新：只更新修改的项目
                let arc_project = Arc::new(updated_project);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_project(arc_project);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "update_project",
                    &project.id,
                );
                error!("{}", context.format_user_message());
            }
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
                info!("Successfully deleted project: {}", project_id);
                // 增量更新：只删除指定的项目
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_project(&project_id);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_project",
                    &project_id,
                );
                error!("{}", context.format_user_message());
            }
        }
    })
    .detach();
}
