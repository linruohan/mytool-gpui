use std::sync::Arc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;
use tracing::{error, info};

use crate::{
    core::error_handler::{AppError, ErrorHandler, validation},
    todo_state::{DBState, TodoStore},
};

// 刷新指定项目的 items（仅在有活跃项目时需要）
async fn refresh_project_items(_project_id: &str, _cx: &mut AsyncApp, _db: DatabaseConnection) {
    // 由于使用了 TodoStore 作为唯一数据源，不再需要单独更新 ProjectState
    // TodoStore 会通过观察者模式自动更新所有视图
}

// 添加 item（使用增量更新，性能最优）
pub fn add_item(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item");
        error!("{}", context.format_user_message());
        // TODO: 显示错误提示给用户
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_item(item.clone(), db.clone()).await {
            Ok(new_item) => {
                info!("Successfully added item: {}", new_item.id);
                // 增量更新：只添加新任务到 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(Arc::new(new_item));
                });
            },
            Err(e) => {
                let context =
                    ErrorHandler::handle_with_resource(AppError::Database(e), "add_item", &item.id);
                error!("{}", context.format_user_message());
                // TODO: 显示错误提示给用户
            },
        }
    })
    .detach();
}

// 修改 item（使用增量更新，性能最优）
pub fn update_item(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item");
        error!("{}", context.format_user_message());
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    let active_project = cx.global::<TodoStore>().active_project.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::mod_item(item.clone(), db.clone()).await {
            Ok(updated_item) => {
                info!("Successfully updated item: {}", updated_item.id);
                // 增量更新：只更新修改的任务
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
                // 如果有活跃项目，刷新项目列表
                if let Some(active) = active_project {
                    refresh_project_items(&active.id, cx, db).await;
                }
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "update_item",
                    &item.id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 删除 item（使用增量更新，性能最优）
pub fn delete_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::del_item(item.clone(), db.clone()).await {
            Ok(_) => {
                info!("Successfully deleted item: {}", item_id);
                // 增量更新：只删除指定的任务
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_item(&item_id);
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_item",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 完成任务（使用增量更新，性能最优）
pub fn completed_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::finish_item(item.clone(), true, false, db.clone()).await {
            Ok(_) => {
                info!("Successfully completed item: {}", item_id);
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.checked = true;
                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "completed_item",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 取消完成任务（使用增量更新，性能最优）
pub fn uncompleted_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::finish_item(item.clone(), false, false, db.clone()).await {
            Ok(_) => {
                info!("Successfully uncompleted item: {}", item_id);
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.checked = false;
                updated_item.completed_at = None;
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "uncompleted_item",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 置顶/取消置顶任务（使用增量更新，性能最优）
pub fn set_item_pinned(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::pin_item(item.clone(), pinned, db.clone()).await {
            Ok(_) => {
                info!(
                    "Successfully {} item: {}",
                    if pinned { "pinned" } else { "unpinned" },
                    item_id
                );
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.pinned = pinned;
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "set_item_pinned",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}
