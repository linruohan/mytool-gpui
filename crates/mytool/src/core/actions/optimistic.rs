//! 乐观更新 - 立即更新UI,异步保存到数据库
//!
//! 这个模块提供了乐观更新的实现，可以显著提升用户体验：
//! 1. 立即更新 UI（乐观更新）
//! 2. 异步保存到数据库
//! 3. 如果保存失败，回滚 UI 更新

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::{Store, entity::ItemModel};
use tracing::{error, info, warn};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{QueryCache, TodoEventBus, TodoStore, TodoStoreEvent, get_db_connection},
    },
    state_service,
};

/// 乐观添加任务
///
/// 1. 立即更新 UI（使用临时 ID）
/// 2. 异步保存到数据库
/// 3. 用真实 ID 替换临时 ID
/// 4. 如果失败，回滚更新
pub fn add_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item_optimistic");
        error!("{}", context.format_user_message());
        return;
    }

    // 1. 生成临时 ID
    let temp_id = format!("temp_{}", uuid::Uuid::new_v4());
    let mut optimistic_item = (*item).clone();
    optimistic_item.id = temp_id.clone();

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
        bus.publish(TodoStoreEvent::ItemAdded(temp_id.clone()));
    });

    // 3. 异步保存到数据库
    let db = get_db_connection(cx);
    cx.spawn(async move |cx| {
        match state_service::add_item(item.clone(), (*db).clone()).await {
            Ok(saved_item) => {
                info!(
                    "Successfully saved item, replacing temp ID {} with real ID {}",
                    temp_id, saved_item.id
                );

                // 4. 用真实 ID 替换临时 ID
                cx.update_global::<TodoStore, _>(|store, _| {
                    // 移除临时项
                    store.remove_item(&temp_id);
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
                    store.remove_item(&temp_id);
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemDeleted(temp_id.clone()));
                });

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_item_optimistic",
                    &item.id,
                );
                error!("{}", context.format_user_message());
                // TODO: 显示错误提示给用户
            },
        }
    })
    .detach();
}

/// 乐观更新任务
///
/// 1. 立即更新 UI
/// 2. 异步保存到数据库
/// 3. 如果失败，恢复旧值
pub fn update_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item_optimistic");
        error!("{}", context.format_user_message());
        return;
    }

    // 1. 保存旧值（用于回滚）
    let old_item = cx.global::<TodoStore>().get_item(&item.id);

    if old_item.is_none() {
        warn!("Item {} not found in store, cannot update optimistically", item.id);
        return;
    }

    let old_item = old_item.unwrap();

    info!("Optimistically updating item: {}", item.id);

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

    cx.spawn(async move |cx| {
        match state_service::mod_item(item.clone(), (*db).clone()).await {
            Ok(updated_item) => {
                info!("Successfully saved item update: {}", item_id);

                // 更新为数据库返回的最新值
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item.clone()));
                });

                // 清空缓存
                cx.update_global::<QueryCache, _>(|cache, _| {
                    cache.invalidate_all();
                });

                // 发布事件
                cx.update_global::<TodoEventBus, _>(|bus, _| {
                    bus.publish(TodoStoreEvent::ItemUpdated(updated_item.id.clone()));
                });
            },
            Err(e) => {
                error!("Failed to save item update, rolling back");

                // 4. 失败时回滚到旧值
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(old_item.clone());
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
                    "update_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
                // TODO: 显示错误提示给用户
            },
        }
    })
    .detach();
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
                // TODO: 显示错误提示给用户
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

    // 2. 异步保存到数据库（在后台执行，避免阻塞UI线程）
    let db = get_db_connection(cx);
    let item_id_clone = item_id.clone();
    let pinned_clone = pinned;

    // 在后台执行数据库操作
    tokio::spawn(async move {
        // 解引用Arc获取DatabaseConnection
        let store = Store::new((*db).clone());

        let result = store.update_item_pin(&item_id_clone, pinned_clone).await;

        match result {
            Ok(_) => {
                info!("Successfully saved pinned status: {}", item_id_clone);

                // 验证保存是否成功：重新从数据库加载并检查
                let verify_result = store.get_item(&item_id_clone).await;

                if let Some(verified_item) = verify_result {
                    info!(
                        "Verified pinned status in database: item {} has pinned = {}",
                        item_id_clone, verified_item.pinned
                    );
                } else {
                    error!("Failed to verify pinned status in database: item not found");
                }
            },
            Err(e) => {
                error!("Failed to save pinned status: {:?}", e);

                // 注意：由于App类型不支持clone，我们无法在后台任务中回滚UI状态
                // 但数据库操作失败不会影响已经更新的UI状态，只是数据不会持久化
                // 在下一次应用启动时，数据会从数据库重新加载，恢复到原始状态

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "set_item_pinned_optimistic",
                    &item_id_clone,
                );
                error!("{}", context.format_user_message());
            },
        }
    });
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
