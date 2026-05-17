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
use tracing::{error, info, warn};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{ErrorNotifier, TodoEventBus, TodoStore, TodoStoreEvent, get_store},
        tokio_runtime::spawn_db_operation,
        utils::retry::{self, RetryConfig},
    },
    state_service,
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
        return "".to_string();
    }

    // 1. 生成临时 ID（用于乐观更新 UI）
    let temp_id = format!("temp_{}", uuid::Uuid::new_v4());
    let temp_id_clone = temp_id.clone();
    let mut optimistic_item = (*item).clone();
    optimistic_item.id = temp_id_clone.clone();

    info!("Optimistically adding item with temp ID: {}, content: '{}'", temp_id, item.content);

    // 2. ⚡ 立即更新 UI（乐观更新，用户无感知延迟）
    cx.update_global::<TodoStore, _>(|store, _| {
        store.add_item(Arc::new(optimistic_item.clone()));
    });

    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemAdded(temp_id_clone.clone()));
    });

    // 3. 🔄 异步保存到数据库（增强版：独立 Runtime + 重试机制）
    let db_state = cx.global::<DBState>().clone();
    let item_clone = item.clone();

    let temp_id_for_async = temp_id.clone();

    cx.spawn(async move |cx| {
        let spawn_start = std::time::Instant::now();
        info!("🚀 [add_item_optimistic] 异步保存任务启动, temp_id={}", temp_id_for_async);

        // 使用 spawn_db_operation 在独立的 Tokio Runtime 中执行 DB 操作
        let save_result = spawn_db_operation(async move {
            info!("📊 [add_item_optimistic] 进入 DB Runtime, 准备获取 Store...");
            let store_ready_start = std::time::Instant::now();

            // 等待 Store 初始化完成（最多 10 秒）
            db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await?;
            info!(
                "📊 [add_item_optimistic] Store 就绪, 耗时={}ms",
                store_ready_start.elapsed().as_millis()
            );

            let store = db_state.get_store_async().await;
            let item_for_save = item_clone.clone();

            // 🔍 诊断：在插入前检查连接池状态
            if let Some(stats) = get_pool_stats(&db_state) {
                info!(
                    "📊 [add_item_optimistic] 连接池状态: idle={}, used={}, max={}",
                    stats.idle, stats.used, stats.max
                );
                if stats.used >= stats.max {
                    warn!("⚠️ [add_item_optimistic] 连接池已满！可能需要等待其他操作释放连接");
                }
            }

            info!("📊 [add_item_optimistic] 开始执行 insert (带重试)...");
            let insert_start = std::time::Instant::now();

            // 使用重试机制执行插入
            let result = retry::retry_async_todo(
                |_attempt| {
                    let store = store.clone();
                    let item = item_for_save.clone();
                    async move { state_service::add_item_with_store(item, store).await }
                },
                RetryConfig::for_db_operation(),
            )
            .await;

            info!(
                "📊 [add_item_optimistic] insert 完成 (含重试), 耗时={}ms, 结果={}",
                insert_start.elapsed().as_millis(),
                if result.is_ok() { "✅" } else { "❌" }
            );

            result
        })
        .await;

        info!("🏁 [add_item_optimistic] 异步任务总耗时={}ms", spawn_start.elapsed().as_millis());

        match save_result {
            Ok(Ok(saved_item)) => {
                let real_id = saved_item.id.clone();
                info!(
                    "✅ Successfully saved item, replacing temp ID {} with real ID {}",
                    temp_id_for_async, real_id
                );

                cx.update_global::<TodoStore, _>(|store, _| {
                    store.replace_item_id(&temp_id_for_async, Arc::new(saved_item));
                });

                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemIdChanged {
                        old_id: temp_id_for_async,
                        new_id: real_id,
                    });
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_item_optimistic",
                    &item.id,
                );
                error!("❌ 添加任务失败（重试耗尽）: {}", context.format_user_message());

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!(
                        "添加任务失败：{}。请稍后重试。",
                        context.format_user_message()
                    ));
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

    temp_id
}

/// 乐观更新任务
pub fn update_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item_optimistic");
        error!("{}", context.format_user_message());
        return;
    }

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(item.clone());
    });

    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item.id.clone()));
    });

    // 为了避免在 Store 未初始化时 panic，异步等待 Store 准备后再执行数据库更新。
    let item_id = item.id.clone();
    let item_for_db = item.clone();
    let db_state = cx.global::<DBState>().clone();

    cx.spawn(async move |cx| {
        // 等待 Store 初始化，最长 10 秒
        if let Err(e) =
            db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await
        {
            error!("❌ 等待 Store 就绪超时: {}", e);
            cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                notifier.set_error("更新任务失败：Store 未就绪，请稍后重试。".to_string());
            });
            return;
        }
        let store = db_state.get_store_async().await;
        let result = state_service::mod_item_with_store(item_for_db.clone(), store).await;
        match result {
            Ok(updated_item) => {
                info!(
                    "Successfully saved item update: {} with priority: {:?}, content: '{}', \
                     due={:?}",
                    item_id, updated_item.priority, updated_item.content, updated_item.due
                );
            },
            Err(e) => {
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
            },
        }
    })
    .detach();
}

/// 乐观删除任务
pub fn delete_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    let item_id = item.id.clone();

    info!("Optimistically deleting item: {}", item_id);

    cx.update_global::<TodoStore, _>(|store, _| {
        store.remove_item(&item_id);
    });

    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemDeleted(item_id.clone()));
    });

    let item_for_recovery = item.clone();
    let store = get_store(cx);
    let item_clone = item.clone();

    cx.spawn(async move |cx| {
        let result = state_service::del_item_with_store(item_clone.clone(), store).await;

        match result {
            Ok(_) => {
                info!("Successfully deleted item from database: {}", item_id);
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

    info!("Optimistically {} item: {}", if pinned { "pinning" } else { "unpinning" }, item_id);

    let mut updated_item = (*item).clone();
    updated_item.pinned = pinned;

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(Arc::new(updated_item.clone()));
    });

    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item_id.clone()));
    });

    let store = get_store(cx);
    let item_id_clone = item_id.clone();

    cx.spawn(async move |cx| {
        let result = store.update_item_pin(&item_id_clone, pinned).await;

        match result {
            Ok(_) => {
                info!("Successfully saved pinned status: {}", item_id);
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

    info!(
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

    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item_id.clone()));
    });

    let store = get_store(cx);
    let item_clone = item.clone();

    cx.spawn(async move |cx| {
        let result = state_service::finish_item_with_store(
            item_clone.clone(),
            checked,
            false,
            store.clone(),
        )
        .await;

        match result {
            Ok(()) => {
                info!("Successfully saved completion status: {}", item_id);
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

/// 获取连接池统计信息（用于诊断）
///
/// 尝试从底层数据库连接中获取连接池状态。
/// 注意：SeaORM/SQLx 的连接池统计 API 可能不直接暴露，
/// 这里提供一个简化版本。
fn get_pool_stats(_db_state: &DBState) -> Option<PoolStats> {
    // TODO: 当 SQLx/SeaORM 提供公开的池统计 API 时实现
    // 目前返回 None，但不影响日志输出
    None
}

/// 连接池统计信息
#[derive(Debug, Clone)]
struct PoolStats {
    idle: usize,
    used: usize,
    max: usize,
}
