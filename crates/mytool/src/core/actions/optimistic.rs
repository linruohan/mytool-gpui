//! 乐观更新 - 立即更新UI,异步保存到数据库
//!
//! 这个模块提供了乐观更新的实现，可以显著提升用户体验：
//! 1. 立即更新 UI（乐观更新）
//! 2. 异步保存到数据库
//! 3. 如果保存失败，回滚 UI 更新

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::{Store, entity::ItemModel};
use tracing::{error, info};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{
            ErrorNotifier, PendingTasksState, QueryCache, TodoEventBus, TodoStore, TodoStoreEvent,
            get_db_connection,
        },
    },
    state_service,
};

/// 乐观添加任务
///
/// 1. 立即更新 UI（使用临时 ID）
/// 2. 异步保存到数据库
/// 3. 用真实 ID 替换临时 ID
/// 4. 如果失败，回滚更新
///
/// # 返回值
/// - 返回生成的临时 ID，用于更新原始 item 对象
pub fn add_item_optimistic(item: Arc<ItemModel>, cx: &mut App) -> String {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item_optimistic");
        error!("{}", context.format_user_message());
        return "".to_string();
    }

    // 1. 生成临时 ID
    let temp_id = format!("temp_{}", uuid::Uuid::new_v4());
    let temp_id_clone = temp_id.clone();
    let mut optimistic_item = (*item).clone();
    optimistic_item.id = temp_id_clone.clone();

    info!("Optimistically adding item with temp ID: {}", temp_id);

    // 2. 立即更新 UI
    cx.update_global::<TodoStore, _>(|store, _| {
        store.add_item(Arc::new(optimistic_item.clone()));
    });

    // 清空缓存
    cx.update_global::<QueryCache, _>(|cache, _| {
        cache.invalidate_all();
    });

    // 🚀 标记受影响的视图为脏
    cx.update_global::<crate::core::state::DirtyFlags, _>(|flags, _| {
        use crate::core::state::{ChangeType, ViewType};

        let change = ChangeType::ItemAdded(Arc::new(optimistic_item.clone()));

        // 标记所有受影响的视图
        if change.affects_view(ViewType::Inbox) {
            flags.mark_dirty(ViewType::Inbox);
        }
        if change.affects_view(ViewType::Today) {
            flags.mark_dirty(ViewType::Today);
        }
        if change.affects_view(ViewType::Scheduled) {
            flags.mark_dirty(ViewType::Scheduled);
        }
        if change.affects_view(ViewType::Pinned) {
            flags.mark_dirty(ViewType::Pinned);
        }
    });

    // 发布事件
    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemAdded(temp_id_clone.clone()));
    });

    // 3. 异步保存到数据库
    let db = get_db_connection(cx);

    // 🚀 跟踪待处理任务
    let task_id = format!("add_item_{}", temp_id);
    cx.update_global::<PendingTasksState, _>(|state, _| {
        state.increment(&task_id);
    });

    // 🚀 使用 tokio::spawn 在 tokio 运行时上执行数据库操作
    let item_clone = item.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    tokio::spawn(async move {
        let result = state_service::add_item(item_clone.clone(), (*db).clone()).await;
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        match rx.await {
            Ok(Ok(saved_item)) => {
                info!(
                    "Successfully saved item, replacing temp ID {} with real ID {}",
                    temp_id_clone, saved_item.id
                );

                // 4. 用真实 ID 替换临时项
                cx.update_global::<TodoStore, _>(|store, _| {
                    // 移除临时项
                    store.remove_item(&temp_id_clone);
                    // 添加真实项
                    store.add_item(Arc::new(saved_item.clone()));
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemUpdated(saved_item.id.clone()));
                });
            },
            Ok(Err(e)) => {
                error!("Failed to save item, rolling back optimistic update");

                // 5. 失败时回滚
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_item(&temp_id_clone);
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemDeleted(temp_id_clone.clone()));
                });

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_item_optimistic",
                    &item.id,
                );
                error!("{}", context.format_user_message());
                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(context.format_user_message());
                });

                // 设置错误状态
                cx.update_global::<PendingTasksState, _>(|state, _| {
                    state.set_error(context.format_user_message());
                });
            },
            Err(_) => {
                error!("❌ Database operation channel closed for item {}", temp_id_clone);
            },
        }

        // 🚀 任务完成，减少计数
        cx.update_global::<PendingTasksState, _>(|state, _| {
            state.decrement(&task_id);
        });
    })
    .detach();

    // 返回临时 ID
    temp_id
}

/// 乐观更新任务
///
/// 1. 立即更新 UI
/// 2. 异步保存到数据库
/// 3. 如果失败，恢复旧值
pub fn update_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    info!("🚀 update_item_optimistic START - item: {}, content: '{}'", item.id, item.content);

    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item_optimistic");
        error!("{}", context.format_user_message());
        return;
    }

    // 2. 立即更新 UI
    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(item.clone());
    });

    // 清空缓存
    cx.update_global::<QueryCache, _>(|cache, _| {
        cache.invalidate_all();
    });

    // 发布事件
    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item.id.clone()));
    });

    // 3. 异步保存到数据库
    let db = get_db_connection(cx);
    let item_id = item.id.clone();
    let _item_priority = item.priority;
    let _item_content = item.content.clone();
    let item_due = item.due.clone();

    info!("🔄 Spawning async task for database save - item: {}, due: {:?}", item_id, item_due);

    // 🚀 跟踪待处理任务
    let task_id = format!("update_item_{}", item_id);
    cx.update_global::<PendingTasksState, _>(|state, _| {
        state.increment(&task_id);
    });

    // 🚀 关键修复：使用 tokio::spawn 在 tokio 运行时上执行数据库操作
    // Sea-ORM 需要 tokio 运行时，而 cx.spawn 运行在 GPUI 的 smol 运行时上
    let item_for_db = item.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    // 在 tokio 运行时上执行数据库操作
    tokio::spawn(async move {
        let result = state_service::mod_item(item_for_db.clone(), (*db).clone()).await;
        let _ = tx.send(result);
    });

    // 在 GPUI 运行时上等待结果并更新状态
    cx.spawn(async move |cx| {
        match rx.await {
            Ok(Ok(updated_item)) => {
                info!(
                    "✅ Successfully saved item update: {} with priority: {:?}, content: '{}', due={:?}",
                    item_id, updated_item.priority, updated_item.content, updated_item.due
                );
                // 保存成功后，更新 TodoStore 中的 item 为数据库返回的最新状态
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Ok(Err(e)) => {
                error!("❌ Failed to save item update for {}, error: {:?}", item_id, e);

                // 设置错误状态
                let error_msg = format!("Failed to save item {}: {:?}", item_id, e);
                cx.update_global::<PendingTasksState, _>(|state, _| {
                    state.set_error(error_msg);
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(format!("保存失败: {}", item_id));
                });
            },
            Err(_) => {
                error!("❌ Database operation channel closed for item {}", item_id);
            },
        }

        // 🚀 任务完成，减少计数
        cx.update_global::<PendingTasksState, _>(|state, _| {
            state.decrement(&task_id);
        });
    })
    .detach();

    info!("🚀 update_item_optimistic END - async task detached");
}

/// 乐观删除任务
///
/// 1. 立即从 UI 移除
/// 2. 异步从数据库删除
/// 3. 如果失败，恢复任务
pub fn delete_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    let item_id = item.id.clone();

    info!("Optimistically deleting item: {}", item_id);

    // 1. 立即从 UI 移除
    cx.update_global::<TodoStore, _>(|store, _| {
        store.remove_item(&item_id);
    });

    // 清空缓存
    cx.update_global::<QueryCache, _>(|cache, _| {
        cache.invalidate_all();
    });

    // 发布事件
    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemDeleted(item_id.clone()));
    });

    // 2. 异步从数据库删除
    let db = get_db_connection(cx);

    // 🚀 跟踪待处理任务
    let task_id = format!("delete_item_{}", item_id);
    cx.update_global::<PendingTasksState, _>(|state, _| {
        state.increment(&task_id);
    });

    // 🚀 使用 tokio::spawn 在 tokio 运行时上执行数据库操作
    let item_clone = item.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    tokio::spawn(async move {
        let result = state_service::del_item(item_clone.clone(), (*db).clone()).await;
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        match rx.await {
            Ok(Ok(_)) => {
                info!("Successfully deleted item from database: {}", item_id);
            },
            Ok(Err(e)) => {
                error!("Failed to delete item from database, restoring");

                // 3. 失败时恢复任务
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(item.clone());
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemAdded(item_id.clone()));
                });

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(context.format_user_message());
                });

                // 设置错误状态
                cx.update_global::<PendingTasksState, _>(|state, _| {
                    state.set_error(context.format_user_message());
                });
            },
            Err(_) => {
                error!("❌ Database operation channel closed for item {}", item_id);
            },
        }

        // 🚀 任务完成，减少计数
        cx.update_global::<PendingTasksState, _>(|state, _| {
            state.decrement(&task_id);
        });
    })
    .detach();
}

/// 乐观设置置顶状态
///
/// 1. 立即更新 UI
/// 2. 同步保存到数据库（确保数据立即持久化）
/// 3. 如果失败，恢复旧值
pub fn set_item_pinned_optimistic(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let _old_pinned = item.pinned;

    info!("Optimistically {} item: {}", if pinned { "pinning" } else { "unpinning" }, item_id);

    // 1. 立即更新 UI
    let mut updated_item = (*item).clone();
    updated_item.pinned = pinned;

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(Arc::new(updated_item.clone()));
    });

    // 清空缓存
    cx.update_global::<QueryCache, _>(|cache, _| {
        cache.invalidate_all();
    });

    // 🚀 标记受影响的视图为脏
    cx.update_global::<crate::core::state::DirtyFlags, _>(|flags, _| {
        use crate::core::state::{ChangeType, ViewType};

        let change =
            ChangeType::ItemUpdated { old: item.clone(), new: Arc::new(updated_item.clone()) };

        // 标记所有受影响的视图
        if change.affects_view(ViewType::Pinned) {
            flags.mark_dirty(ViewType::Pinned);
        }
    });

    // 发布事件
    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item_id.clone()));
    });

    // 2. 异步保存到数据库（使用 cx.spawn 确保应用在关闭前等待任务完成）
    let db = get_db_connection(cx);

    // 🚀 跟踪待处理任务
    let task_id = format!("pin_item_{}", item_id);
    cx.update_global::<PendingTasksState, _>(|state, _| {
        state.increment(&task_id);
    });

    // 🚀 使用 tokio::spawn 在 tokio 运行时上执行数据库操作
    let item_id_for_db = item_id.clone();
    let item_id_for_log = item_id.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    tokio::spawn(async move {
        let store = Store::new((*db).clone());
        let result = store.update_item_pin(&item_id_for_db, pinned).await;
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        match rx.await {
            Ok(Ok(_)) => {
                info!("Successfully saved pinned status: {}", item_id_for_log);
            },
            Ok(Err(e)) => {
                error!("Failed to save pinned status: {:?}", e);

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "set_item_pinned_optimistic",
                    &item_id_for_log,
                );
                error!("{}", context.format_user_message());

                // 设置错误状态
                cx.update_global::<PendingTasksState, _>(|state, _| {
                    state.set_error(context.format_user_message());
                });
            },
            Err(_) => {
                error!("❌ Database operation channel closed for item {}", item_id_for_log);
            },
        }

        // 🚀 任务完成，减少计数
        cx.update_global::<PendingTasksState, _>(|state, _| {
            state.decrement(&task_id);
        });
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

    // 1. 立即更新 UI
    let mut updated_item = (*item).clone();
    updated_item.checked = checked;
    updated_item.completed_at = if checked { Some(chrono::Utc::now().naive_utc()) } else { None };

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(Arc::new(updated_item.clone()));
    });

    // 清空缓存
    cx.update_global::<QueryCache, _>(|cache, _| {
        cache.invalidate_all();
    });

    // 发布事件
    cx.update_global::<TodoEventBus, _>(|bus, _| {
        bus.publish(TodoStoreEvent::ItemUpdated(item_id.clone()));
    });

    // 2. 异步保存到数据库
    let db = get_db_connection(cx);

    // 🚀 跟踪待处理任务
    let task_id = format!("complete_item_{}", item_id);
    cx.update_global::<PendingTasksState, _>(|state, _| {
        state.increment(&task_id);
    });

    // 🚀 使用 tokio::spawn 在 tokio 运行时上执行数据库操作
    let item_clone = item.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    tokio::spawn(async move {
        let result =
            state_service::finish_item(item_clone.clone(), checked, false, (*db).clone()).await;
        let _ = tx.send(result);
    });

    cx.spawn(async move |cx| {
        match rx.await {
            Ok(Ok(_)) => {
                info!("Successfully saved completion status: {}", item_id);
            },
            Ok(Err(e)) => {
                error!("Failed to save completion status, rolling back");

                // 3. 失败时回滚
                let mut rollback_item = (*item).clone();
                rollback_item.checked = old_checked;
                rollback_item.completed_at =
                    if old_checked { Some(chrono::Utc::now().naive_utc()) } else { None };

                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(rollback_item));
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemUpdated(item_id.clone()));
                });

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "complete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                // 设置错误状态
                cx.update_global::<PendingTasksState, _>(|state, _| {
                    state.set_error(context.format_user_message());
                });
            },
            Err(_) => {
                error!("❌ Database operation channel closed for item {}", item_id);
            },
        }

        // 🚀 任务完成，减少计数
        cx.update_global::<PendingTasksState, _>(|state, _| {
            state.decrement(&task_id);
        });
    })
    .detach();
}
