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

/// 乐观添加项目：先写入内存并使用稳定 UUID，落盘时保留同一主键。
pub fn add_project(project: Arc<ProjectModel>, cx: &mut App) {
    if let Err(e) = validation::validate_project_name(&project.name) {
        let context = ErrorHandler::handle_with_location(e, "add_project");
        error!("{}", context.format_user_message());
        return;
    }

    let project_id = if project.id.is_empty() || project.id.starts_with("temp_") {
        uuid::Uuid::new_v4().to_string()
    } else {
        project.id.clone()
    };
    let persist_project =
        Arc::new(ProjectModel { id: project_id.clone(), ..project.as_ref().clone() });
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.add_project(persist_project.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let persist_project = persist_project.clone();
                move |store| async move { store.insert_project(persist_project.as_ref().clone()).await }
            })
            .await
        {
            Ok(Ok(new_project)) => {
                debug!("Successfully added project: {}", new_project.id);
                let arc_project = Arc::new(new_project);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(arc_project);
                });
            },
            Ok(Err(e)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_project(&project_id);
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
                    todo_store.remove_project(&project_id);
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

/// 批量更新项目顺序（先写内存，再逐条落盘）。
pub fn batch_update_projects(projects: Vec<Arc<ProjectModel>>, cx: &mut App) {
    if projects.is_empty() {
        return;
    }
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        for project in &projects {
            todo_store.update_project(project.clone());
        }
    });
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        for project in projects {
            match db_state
                .spawn_store_op({
                    let project = project.clone();
                    move |store| async move { store.update_project(project.as_ref().clone()).await }
                })
                .await
            {
                Ok(Ok(updated)) => {
                    cx.update_global::<TodoStore, _>(|todo_store, _| {
                        todo_store.update_project(Arc::new(updated));
                    });
                },
                Ok(Err(e)) => error!("batch_update_projects failed: {:?}", e),
                Err(join_err) => error!("batch_update_projects task panicked: {:?}", join_err),
            }
        }
    })
    .detach();
}

/// 拖放到目标项目后，按同级关系重写 `child_order` 并保存。
pub fn drop_reorder_projects(from_id: &str, to_id: &str, cx: &mut App) {
    let updated = {
        let store = cx.global::<TodoStore>();
        crate::todo_state::reorder_drop_among_projects(&store.projects, from_id, to_id)
    };
    let Some(updated) = updated else {
        return;
    };
    batch_update_projects(updated, cx);
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

pub fn set_project_collapsed(project: Arc<ProjectModel>, collapsed: bool, cx: &mut App) {
    if project.collapsed == collapsed {
        return;
    }
    let original = project.clone();
    let mut updated = (*project).clone();
    updated.collapsed = collapsed;
    let after = Arc::new(updated);
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.update_project(after.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let after = after.clone();
                move |store| async move { store.update_project(after.as_ref().clone()).await }
            })
            .await
        {
            Ok(Ok(saved)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(Arc::new(saved));
                });
            },
            Ok(Err(e)) => {
                error!("set_project_collapsed failed: {:?}", e);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(original.clone());
                });
            },
            Err(join_err) => {
                error!("set_project_collapsed task panicked: {:?}", join_err);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(original);
                });
            },
        }
    })
    .detach();
}

pub fn set_project_favorite(project: Arc<ProjectModel>, is_favorite: bool, cx: &mut App) {
    if project.is_favorite == is_favorite {
        return;
    }
    let original = project.clone();
    let mut updated = (*project).clone();
    updated.is_favorite = is_favorite;
    let after = Arc::new(updated);
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.update_project(after.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let after = after.clone();
                move |store| async move { store.update_project(after.as_ref().clone()).await }
            })
            .await
        {
            Ok(Ok(saved)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(Arc::new(saved));
                });
            },
            Ok(Err(e)) => {
                error!("set_project_favorite failed: {:?}", e);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(original.clone());
                });
            },
            Err(join_err) => {
                error!("set_project_favorite task panicked: {:?}", join_err);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_project(original);
                });
            },
        }
    })
    .detach();
}
