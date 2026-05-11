use std::sync::Arc;

use gpui::{App, AsyncApp};
use todos::entity::ItemModel;
use tracing::{error, info};

use crate::core::{
    error_handler::{AppError, ErrorHandler, validation},
    state::{ErrorNotifier, TodoStore, get_store},
};

// 刷新项目 items（由于使用了 TodoStore 作为唯一数据源，不再需要单独更新）
async fn refresh_project_items(_project_id: &str, _cx: &mut AsyncApp) {
    // TodoStore 会通过观察者模式自动更新所有视图
}

/// 统一错误处理辅助函数
///
/// 将数据库错误转换为统一格式并通知用户
fn handle_db_error(
    cx: &mut AsyncApp,
    operation: &str,
    entity_id: &str,
    e: todos::error::TodoError,
) {
    let context = ErrorHandler::handle_with_resource(AppError::Database(e), operation, entity_id);
    error!("{}", context.format_user_message());
    cx.update_global::<ErrorNotifier, _>(|notifier, _| {
        notifier.set_error(context.format_user_message());
    });
}

/// 添加 item（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一为 cx.spawn 异步模式，避免同步阻塞 UI 线程
pub fn add_item(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item");
        error!("{}", context.format_user_message());
        return;
    }

    let store = get_store(cx);
    let item_clone = item.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::add_item_with_store(item_clone, store).await {
            Ok(new_item) => {
                info!("Successfully added item: {}", new_item.id);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_item(Arc::new(new_item));
                });
            },
            Err(e) => {
                handle_db_error(cx, "add_item", &item_id, e);
            },
        }
    })
    .detach();
}

/// 修改 item（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一异步模式，使用 DB 返回的最新数据更新内存
pub fn update_item(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item");
        error!("{}", context.format_user_message());
        return;
    }

    let store = get_store(cx);
    let active_project = cx.global::<TodoStore>().active_project.clone();
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::mod_item_with_store(item.clone(), store).await {
            Ok(updated_item) => {
                info!("Successfully updated item: {}", updated_item.id);
                // 🚀 7.x修复：使用 DB 返回的最新数据，避免内存不一致
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_item(Arc::new(updated_item));
                });
                // 如果有活跃项目，刷新项目列表
                if let Some(active) = active_project {
                    refresh_project_items(&active.id, cx).await;
                }
            },
            Err(e) => {
                handle_db_error(cx, "update_item", &item_id, e);
            },
        }
    })
    .detach();
}

/// 删除 item（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一为 cx.spawn 异步模式
pub fn delete_item(item: Arc<ItemModel>, cx: &mut App) {
    let store = get_store(cx);
    let item_id = item.id.clone();
    let item_clone = item.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::del_item_with_store(item_clone, store).await {
            Ok(_) => {
                info!("Successfully deleted item: {}", item_id);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_item(&item_id);
                });
            },
            Err(e) => {
                handle_db_error(cx, "delete_item", &item_id, e);
            },
        }
    })
    .detach();
}

/// 完成任务（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一异步模式
/// 🚀 7.x修复：始终使用 DB 返回的最新数据，避免内存不一致
pub fn completed_item(item: Arc<ItemModel>, cx: &mut App) {
    let store = get_store(cx);
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::finish_item_with_store(item.clone(), true, false, store.clone())
            .await
        {
            Ok(()) => {
                info!("Successfully completed item: {}", item_id);
                // 🚀 7.x修复：始终从 DB 拉取最新数据，避免内存不一致
                match store.get_item(&item_id).await {
                    Some(fresh) => {
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(fresh));
                        });
                    },
                    None => {
                        tracing::warn!(
                            "completed_item: DB ok but get_item returned None for {}",
                            item_id
                        );
                        // 降级方案：使用传入 item 的修改版本（但可能不完全一致）
                        let mut updated_item = (*item).clone();
                        updated_item.checked = true;
                        updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(updated_item));
                        });
                    },
                }
            },
            Err(e) => {
                handle_db_error(cx, "completed_item", &item_id, e);
            },
        }
    })
    .detach();
}

/// 取消完成任务（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一异步模式
/// 🚀 7.x修复：始终使用 DB 返回的最新数据
pub fn uncompleted_item(item: Arc<ItemModel>, cx: &mut App) {
    let store = get_store(cx);
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::finish_item_with_store(
            item.clone(),
            false,
            false,
            store.clone(),
        )
        .await
        {
            Ok(()) => {
                info!("Successfully uncompleted item: {}", item_id);
                // 🚀 7.x修复：始终从 DB 拉取最新数据
                match store.get_item(&item_id).await {
                    Some(fresh) => {
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(fresh));
                        });
                    },
                    None => {
                        tracing::warn!(
                            "uncompleted_item: DB ok but get_item returned None for {}",
                            item_id
                        );
                        let mut updated_item = (*item).clone();
                        updated_item.checked = false;
                        updated_item.completed_at = None;
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(updated_item));
                        });
                    },
                }
            },
            Err(e) => {
                handle_db_error(cx, "uncompleted_item", &item_id, e);
            },
        }
    })
    .detach();
}

/// 置顶/取消置顶任务（异步执行，统一写路径）
///
/// 🚀 6.3优化：统一异步模式
/// 🚀 7.x修复：始终使用 DB 返回的最新数据
pub fn set_item_pinned(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let store = get_store(cx);
    let item_id = item.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::pin_item_with_store(item.clone(), pinned, store.clone()).await {
            Ok(()) => {
                info!(
                    "Successfully {} item: {}",
                    if pinned { "pinned" } else { "unpinned" },
                    item_id
                );
                // 🚀 7.x修复：始终从 DB 拉取最新数据
                match store.get_item(&item_id).await {
                    Some(fresh) => {
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(fresh));
                        });
                    },
                    None => {
                        tracing::warn!(
                            "set_item_pinned: DB ok but get_item returned None for {}",
                            item_id
                        );
                        let mut updated_item = (*item).clone();
                        updated_item.pinned = pinned;
                        cx.update_global::<TodoStore, _>(|todo_store, _| {
                            todo_store.update_item(Arc::new(updated_item));
                        });
                    },
                }
            },
            Err(e) => {
                handle_db_error(cx, "set_item_pinned", &item_id, e);
            },
        }
    })
    .detach();
}
