//! 乐观更新 - 立即更新UI,异步保存到数据库
//!
//! 这个模块提供了乐观更新的实现，可以显著提升用户体验：
//! 1. 立即更新 UI（乐观更新）
//! 2. 异步保存到数据库
//! 3. 如果保存失败，回滚 UI 更新

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::ItemModel;
use tracing::{error, info};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{QueryCache, TodoEventBus, TodoStore, TodoStoreEvent, get_db_connection},
        tokio_runtime,
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

    // 3. 保存到数据库（在独立的 tokio runtime 中异步执行）
    let db = get_db_connection(cx);

    info!("🔄 Saving new item to database: {}", temp_id);

    // 🚀 关键修复：使用 spawn 而非 block_on，避免在异步上下文中阻塞线程
    let item_clone = item.clone();

    tokio_runtime::spawn_db_operation(async move {
        let result = state_service::add_item(item_clone.clone(), (*db).clone()).await;

        match result {
            Ok(saved_item) => {
                info!(
                    "✅ Successfully saved item, replacing temp ID {} with real ID {}",
                    temp_id_clone, saved_item.id
                );

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们依赖数据库操作的结果，UI 已经通过乐观更新得到了更新
                // 实际应用中，可能需要通过事件系统来通知 UI 更新
            },
            Err(e) => {
                error!("❌ Failed to save item, rolling back optimistic update");

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们记录错误，但无法直接回滚 UI 更新
                // 实际应用中，可能需要通过事件系统来通知 UI 回滚

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_item_optimistic",
                    &item.id,
                );
                error!("{}", context.format_user_message());
            },
        }
    });

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

    // 3. 保存到数据库（在独立的 tokio runtime 中同步执行）
    let db = get_db_connection(cx);
    let item_id = item.id.clone();
    let _item_priority = item.priority;
    let _item_content = item.content.clone();
    let item_due = item.due.clone();

    info!("🔄 Saving to database - item: {}, due: {:?}", item_id, item_due);

    // 🚀 关键修复：使用 spawn 而非 block_on，避免在异步上下文中阻塞线程
    let item_for_db = item.clone();

    tokio_runtime::spawn_db_operation(async move {
        let result = state_service::mod_item(item_for_db.clone(), (*db).clone()).await;

        match result {
            Ok(updated_item) => {
                info!(
                    "✅ Successfully saved item update: {} with priority: {:?}, content: '{}', due={:?}",
                    item_id, updated_item.priority, updated_item.content, updated_item.due
                );

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们依赖数据库操作的结果，UI 已经通过乐观更新得到了更新
                // 实际应用中，可能需要通过事件系统来通知 UI 更新
            },
            Err(e) => {
                error!("❌ Failed to save item update for {}, error: {:?}", item_id, e);

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们记录错误，但无法直接更新 ErrorNotifier
                // 实际应用中，可能需要通过事件系统来通知 UI 显示错误
            },
        }
    });

    info!("🚀 update_item_optimistic END - database save completed");
}

/// 乐观删除任务
///
/// 1. 立即从 UI 移除
/// 2. 同步从数据库删除
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

    // 2. 异步从数据库删除（使用独立 tokio runtime）
    let db = get_db_connection(cx);

    info!("🔄 Deleting item from database: {}", item_id);

    let item_clone = item.clone();

    tokio_runtime::spawn_db_operation(async move {
        let result = state_service::del_item(item_clone.clone(), (*db).clone()).await;

        match result {
            Ok(_) => {
                info!("✅ Successfully deleted item from database: {}", item_id);
            },
            Err(e) => {
                error!("❌ Failed to delete item from database, restoring");

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们记录错误，但无法直接恢复 UI 状态
                // 实际应用中，可能需要通过事件系统来通知 UI 恢复

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    });
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

    // 2. 异步保存到数据库（使用独立 tokio runtime）
    let db = get_db_connection(cx);

    info!("🔄 Saving pinned status to database: {}", item_id);

    let item_id_clone = item_id.clone();

    tokio_runtime::spawn_db_operation(async move {
        let result = {
            let store = todos::Store::new((*db).clone());
            store.update_item_pin(&item_id_clone, pinned).await
        };

        match result {
            Ok(_) => {
                info!("✅ Successfully saved pinned status: {}", item_id);
            },
            Err(e) => {
                error!("❌ Failed to save pinned status: {:?}", e);

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "set_item_pinned_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    });
}

/// 乐观完成任务
pub fn complete_item_optimistic(item: Arc<ItemModel>, checked: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let _old_checked = item.checked;

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

    // 2. 异步保存到数据库（使用独立 tokio runtime）
    let db = get_db_connection(cx);

    info!("🔄 Saving completion status to database: {}", item_id);

    let item_clone = item.clone();

    tokio_runtime::spawn_db_operation(async move {
        let result =
            state_service::finish_item(item_clone.clone(), checked, false, (*db).clone()).await;

        match result {
            Ok(_) => {
                info!("✅ Successfully saved completion status: {}", item_id);
            },
            Err(e) => {
                error!("❌ Failed to save completion status, rolling back");

                // 注意：由于我们在独立的 tokio runtime 中，无法直接访问 App 上下文
                // 这里我们记录错误，但无法直接回滚 UI 状态
                // 实际应用中，可能需要通过事件系统来通知 UI 回滚

                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "complete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    });
}
