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
            ErrorNotifier, QueryCache, TodoEventBus, TodoStore, TodoStoreEvent, get_db_connection,
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
    cx.spawn(async move |cx| {
        match state_service::add_item(item.clone(), (*db).clone()).await {
            Ok(saved_item) => {
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
            Err(e) => {
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
            },
        }
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
    let item_content = item.content.clone();

    info!("🔄 Spawning async task for database save - item: {}", item_id);

    // 🚀 关键修复：使用 cx.spawn 而不是 tokio::spawn
    // 这样 GPUI 可以在应用关闭前等待这些异步任务完成，避免数据丢失
    let item_for_db = item.clone();
    cx.spawn(async move |cx| {
        info!(
            "⏳ Async task STARTED - Saving to database: item={}, content='{}'",
            item_id, item_content
        );
        match state_service::mod_item(item_for_db.clone(), (*db).clone()).await {
            Ok(updated_item) => {
                info!(
                    "✅ Successfully saved item update: {} with priority: {:?}, content: '{}'",
                    item_id, updated_item.priority, updated_item.content
                );
                // 保存成功后，更新 TodoStore 中的 item 为数据库返回的最新状态
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                error!("❌ Failed to save item update for {}, error: {:?}", item_id, e);
                // 保存失败时，可以在这里添加错误处理逻辑
                // 例如：回滚 UI 状态或显示错误提示
            },
        }
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

    cx.spawn(async move |cx| {
        match state_service::del_item(item.clone(), (*db).clone()).await {
            Ok(_) => {
                info!("Successfully deleted item from database: {}", item_id);
            },
            Err(e) => {
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
            },
        }
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
    let item_id_clone = item_id.clone();

    cx.spawn(async move |_cx| {
        let store = Store::new((*db).clone());

        let result = store.update_item_pin(&item_id_clone, pinned).await;

        match result {
            Ok(_) => {
                info!("Successfully saved pinned status: {}", item_id_clone);
            },
            Err(e) => {
                error!("Failed to save pinned status: {:?}", e);

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "set_item_pinned_optimistic",
                    &item_id_clone,
                );
                error!("{}", context.format_user_message());
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

    cx.spawn(async move |cx| {
        match state_service::finish_item(item.clone(), checked, false, (*db).clone()).await {
            Ok(_) => {
                info!("Successfully saved completion status: {}", item_id);
            },
            Err(e) => {
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
            },
        }
    })
    .detach();
}
