use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::ProjectModel;
use tracing::{error, info};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{TodoStore, get_store},
    },
    todo_state::DBState,
};

/// 乐观添加项目：先写入内存，落盘成功后替换为真实 ID。
pub fn add_project(project: Arc<ProjectModel>, cx: &mut App) {
    if let Err(e) = validation::validate_project_name(&project.name) {
        let context = ErrorHandler::handle_with_location(e, "add_project");
        error!("{}", context.format_user_message());
        return;
    }

    let temp_id = format!("temp_project_{}", uuid::Uuid::new_v4());
    let temp_project = Arc::new(ProjectModel { id: temp_id.clone(), ..project.as_ref().clone() });
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.add_project(temp_project);
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        if let Err(e) =
            db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await
        {
            error!("add_project: Store 未就绪: {}", e);
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.remove_project(&temp_id);
            });
            return;
        }
        let store = db_state.get_store_async().await;
        match store.insert_project(project.as_ref().clone()).await {
            Ok(new_project) => {
                info!("Successfully added project: {}", new_project.id);
                let arc_project = Arc::new(new_project);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.replace_project_id(&temp_id, arc_project);
                });
            },
            Err(e) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_project(&temp_id);
                });
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_project",
                    &project.id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

/// 乐观修改项目
pub fn update_project(project: Arc<ProjectModel>, cx: &mut App) {
    if let Err(e) = validation::validate_project_name(&project.name) {
        let context = ErrorHandler::handle_with_location(e, "update_project");
        error!("{}", context.format_user_message());
        return;
    }

    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.update_project(project.clone());
    });

    let store = get_store(cx);
    cx.spawn(async move |cx| match store.update_project(project.as_ref().clone()).await {
        Ok(updated_project) => {
            info!("Successfully updated project: {}", updated_project.id);
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.update_project(Arc::new(updated_project));
            });
        },
        Err(e) => {
            let context = ErrorHandler::handle_with_resource(
                AppError::Database(Box::new(e)),
                "update_project",
                &project.id,
            );
            error!("{}", context.format_user_message());
        },
    })
    .detach();
}

/// 乐观删除项目，失败时恢复到内存列表
pub fn delete_project(project: Arc<ProjectModel>, cx: &mut App) {
    let snapshot = project.clone();
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.remove_project(&project.id);
    });

    let store = get_store(cx);
    cx.spawn(async move |cx| match store.delete_project(&project.id).await {
        Ok(_) => {
            info!("Successfully deleted project: {}", project.id);
        },
        Err(e) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.add_project(snapshot);
            });
            let context = ErrorHandler::handle_with_resource(
                AppError::Database(Box::new(e)),
                "delete_project",
                &project.id,
            );
            error!("{}", context.format_user_message());
        },
    })
    .detach();
}
