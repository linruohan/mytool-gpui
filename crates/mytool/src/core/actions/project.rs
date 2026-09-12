use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::ProjectModel;
use tracing::{debug, error};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::TodoStore,
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
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.insert_project(project.as_ref().clone()).await
            })
            .await
        {
            Ok(Ok(new_project)) => {
                debug!("Successfully added project: {}", new_project.id);
                let arc_project = Arc::new(new_project);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.replace_project_id(&temp_id, arc_project);
                });
            },
            Ok(Err(e)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_project(&temp_id);
                });
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_project",
                    &project_id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_project(&temp_id);
                });
                error!("add_project task panicked: {:?}", join_err);
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

    let db_state = cx.global::<DBState>().clone();
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.update_project(project.as_ref().clone()).await
            })
            .await
        {
            Ok(Ok(updated_project)) => {
                debug!("Successfully updated project: {}", updated_project.id);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(Arc::new(updated_project));
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "update_project",
                    &project_id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                error!("update_project task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

/// 乐观删除项目，失败时恢复到内存列表
pub fn delete_project(project: Arc<ProjectModel>, cx: &mut App) {
    let snapshot = project.clone();
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.remove_project(&project.id);
    });

    let db_state = cx.global::<DBState>().clone();
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let project_id = project_id.clone();
                move |store| async move { store.delete_project(&project_id).await }
            })
            .await
        {
            Ok(Ok(_)) => {
                debug!("Successfully deleted project: {}", project_id);
            },
            Ok(Err(e)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_project(snapshot);
                });
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "delete_project",
                    &project_id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_project(snapshot);
                });
                error!("delete_project task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}
