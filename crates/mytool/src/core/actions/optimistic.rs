//! 乐观更新 - 立即更新 UI，异步保存到数据库
//!
//! 这个模块提供了乐观更新的实现，可以显著提升用户体验：
//! 1. 立即更新 UI（乐观更新）
//! 2. 异步保存到数据库（使用 cx.spawn + spawn_db_operation，不阻塞 UI）
//! 3. 自动重试机制 + Store 就绪等待，确保数据可靠落盘
//! 4. 窗口关闭时会等待 DB 操作完成后再退出

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::ItemModel;
use tracing::{debug, error};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{ErrorNotifier, TodoStore, get_store},
        tokio_runtime::spawn_db_operation,
        utils::retry::{self, RetryConfig},
    },
    todo_state::DBState,
};

/// 乐观添加任务
///
/// 1. 立即更新 UI（使用临时 ID）
/// 2. 异步保存到数据库（使用共享连接池 + 重试机制）
/// 3. 自动等待 Store 就绪 + 重试机制，确保数据可靠落盘
///
/// # 返回值
/// - 返回生成的临时 ID，用于更新原始 item 对象
pub fn add_item_optimistic(item: Arc<ItemModel>, cx: &mut App) -> String {
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item_optimistic");
        error!("{}", context.format_user_message());
        cx.update_global::<ErrorNotifier, _>(|notifier, _| {
            notifier.set_error(context.format_user_message());
        });
        return "".to_string();
    }

    // 1. 生成稳定 ID：UI 与数据库使用同一主键，避免随后按 temp_ ID UPDATE 失败
    let item_id = if item.id.is_empty() || item.id.starts_with("temp_") {
        uuid::Uuid::new_v4().to_string()
    } else {
        item.id.clone()
    };
    let mut optimistic_item = (*item).clone();
    optimistic_item.id = item_id.clone();

    debug!("Optimistically adding item with ID: {}, content: '{}'", item_id, item.content);

    cx.update_global::<TodoStore, _>(|store, _| {
        store.add_item(Arc::new(optimistic_item.clone()));
    });

    let db_state = cx.global::<DBState>().clone();
    let db_state_for_labels = db_state.clone();
    let item_for_save = Arc::new(optimistic_item);
    let item_id_for_error = item_id.clone();
    let item_id_for_async = item_id.clone();

    cx.spawn(async move |cx| {
        let spawn_start = std::time::Instant::now();
        let save_result = spawn_db_operation(async move {
            db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await?;
            let store = db_state.get_store_async().await;
            retry::retry_async_todo(
                |_attempt| {
                    let store = store.clone();
                    let item = item_for_save.clone();
                    async move { store.insert_item(item.as_ref().clone(), true).await }
                },
                RetryConfig::for_db_operation(),
            )
            .await
        })
        .await;

        debug!(
            "add_item_optimistic finished id={} elapsed_ms={}",
            item_id_for_async,
            spawn_start.elapsed().as_millis()
        );

        match save_result {
            Ok(Ok(saved_item)) => {
                debug!("Successfully saved item with ID {}", saved_item.id);

                // ID 已在插入前确定；若数据库回写了相同记录，仍同步一次内存态
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(saved_item.clone()));
                });

                let label_ids: Vec<String> = saved_item
                    .labels
                    .as_deref()
                    .unwrap_or("")
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if !label_ids.is_empty() {
                    let item_id = saved_item.id.clone();
                    match spawn_db_operation(async move {
                        let store = db_state_for_labels.get_store_async().await;
                        store.set_item_labels(&item_id, &label_ids).await
                    })
                    .await
                    {
                        Ok(Ok(())) => {
                            debug!("Labels saved for new item {}", saved_item.id);
                        },
                        Ok(Err(e)) => {
                            error!("❌ 新建任务后写入标签失败: {}", e);
                        },
                        Err(e) => {
                            error!("❌ 新建任务标签任务异常: {:?}", e);
                        },
                    }
                }

                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_succeeded(saved_item.id);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_item_optimistic",
                    &item_id_for_error,
                );
                error!("❌ 添加任务失败（重试耗尽）: {}", context.format_user_message());

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "添加任务失败：{}。请稍后重试。",
                        context.format_user_message()
                    ));
                });

                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id_for_async.clone());
                });
            },
            Err(join_err) => {
                error!("❌ 添加任务异常（任务被取消或 panic）: {:?}", join_err);

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error("添加任务时发生内部错误。请稍后重试。".to_string());
                });
            },
        }
    })
    .detach();

    item_id
}

/// 乐观更新任务：先写 TodoStore，再在 DB runtime 中落盘。
pub fn update_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item_optimistic");
        error!("{}", context.format_user_message());
        cx.update_global::<ErrorNotifier, _>(|notifier, _| {
            notifier.set_error(context.format_user_message());
        });
        return;
    }

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(item.clone());
    });

    let item_id = item.id.clone();
    let item_for_db = item.clone();
    let db_state = cx.global::<DBState>().clone();

    cx.spawn(async move |cx| {
        let save_result = spawn_db_operation(async move {
            db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await?;
            let store = db_state.get_store_async().await;
            retry::retry_async_todo(
                |_attempt| {
                    let store = store.clone();
                    let item = item_for_db.clone();
                    async move { store.update_item(item.as_ref().clone(), "").await }
                },
                RetryConfig::for_db_operation(),
            )
            .await
        })
        .await;

        match save_result {
            Ok(Ok(updated_item)) => {
                debug!(
                    "Successfully saved item update: {} with priority: {:?}, content: '{}'",
                    item_id, updated_item.priority, updated_item.content
                );
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_succeeded(item_id);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "update_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "更新任务失败：{}。您的更改已保存到本地，稍后会自动重试。",
                        context.format_user_message()
                    ));
                });
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id);
                });
            },
            Err(join_err) => {
                error!("Item update task panicked: {:?}", join_err);
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id);
                });
            },
        }
    })
    .detach();
}

/// 乐观删除任务
pub fn delete_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    let item_id = item.id.clone();

    debug!("Optimistically deleting item: {}", item_id);

    cx.update_global::<TodoStore, _>(|store, _| {
        store.remove_item(&item_id);
    });

    let item_for_recovery = item.clone();
    let store = get_store(cx);

    cx.spawn(async move |cx| {
        let result = store.delete_item(&item_id).await;

        match result {
            Ok(_) => {
                debug!("Successfully deleted item from database: {}", item_id);
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "delete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(item_for_recovery.clone());
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "删除任务失败：{}。任务已恢复到列表中，请稍后重试。",
                        context.format_user_message()
                    ));
                });
            },
        }
    })
    .detach();
}

/// 乐观设置置顶状态
pub fn set_item_pinned_optimistic(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let old_pinned = item.pinned;

    debug!("Optimistically {} item: {}", if pinned { "pinning" } else { "unpinning" }, item_id);

    let mut updated_item = (*item).clone();
    updated_item.pinned = pinned;

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(Arc::new(updated_item.clone()));
    });

    let store = get_store(cx);
    let item_id_clone = item_id.clone();

    cx.spawn(async move |cx| {
        let result = store.update_item_pin(&item_id_clone, pinned).await;

        match result {
            Ok(_) => {
                debug!("Successfully saved pinned status: {}", item_id);
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "set_item_pinned_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                let mut reverted_item = updated_item.clone();
                reverted_item.pinned = old_pinned;
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(reverted_item));
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "{}任务失败：{}。状态已恢复，请稍后重试。",
                        if pinned { "置顶" } else { "取消置顶" },
                        context.format_user_message()
                    ));
                });
            },
        }
    })
    .detach();
}

/// 乐观完成任务
pub fn complete_item_optimistic(item: Arc<ItemModel>, checked: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let old_checked = item.checked;

    debug!(
        "Optimistically {} item: {}",
        if checked { "completing" } else { "uncompleting" },
        item_id
    );

    let mut updated_item = (*item).clone();
    updated_item.checked = checked;
    updated_item.completed_at = if checked { Some(chrono::Utc::now().naive_utc()) } else { None };

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(Arc::new(updated_item.clone()));
    });

    let store = get_store(cx);

    cx.spawn(async move |cx| {
        let result = store.complete_item(&item_id, checked, false).await;

        match result {
            Ok(()) => {
                debug!("Successfully saved completion status: {}", item_id);
                if let Some(fresh) = store.get_item(&item_id).await {
                    cx.update_global::<TodoStore, _>(|todo_store, _| {
                        todo_store.update_item(Arc::new(fresh));
                    });
                }
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "complete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                let mut reverted_item = updated_item.clone();
                reverted_item.checked = old_checked;
                reverted_item.completed_at =
                    if old_checked { Some(chrono::Utc::now().naive_utc()) } else { None };
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(reverted_item));
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "{}任务失败：{}。状态已恢复，请稍后重试。",
                        if checked { "完成" } else { "取消完成" },
                        context.format_user_message()
                    ));
                });
            },
        }
    })
    .detach();
}
