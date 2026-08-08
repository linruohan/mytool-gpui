use std::sync::Arc;

use todos::{Store, entity::ItemModel, error::TodoError};

// ==================== 加载任务 ====================

/// 使用全局 Store 加载 items（推荐）
pub async fn load_items_with_store(store: Arc<Store>) -> Result<Vec<ItemModel>, TodoError> {
    store.get_all_items().await
}

// ==================== 添加任务 ====================

/// 添加任务（推荐）
pub async fn add_item_with_store(
    item: Arc<ItemModel>,
    store: Arc<Store>,
) -> Result<ItemModel, TodoError> {
    tracing::info!(
        "🔗 [state_service::add_item_with_store] 调用 Store::insert_item, content='{}'",
        item.content
    );

    let result = store.insert_item(item.as_ref().clone(), true).await;

    tracing::info!(
        "🔗 [state_service::add_item_with_store] Store::insert_item 返回, 结果={}",
        if result.is_ok() { "✅" } else { "❌" }
    );

    result
}

// ==================== 修改任务 ====================

/// 修改任务（推荐）
pub async fn mod_item_with_store(
    item: Arc<ItemModel>,
    store: Arc<Store>,
) -> Result<ItemModel, TodoError> {
    store.update_item(item.as_ref().clone(), "").await
}

// ==================== 删除任务 ====================

/// 删除任务（推荐）
pub async fn del_item_with_store(item: Arc<ItemModel>, store: Arc<Store>) -> Result<(), TodoError> {
    store.delete_item(&item.id).await
}

// ==================== 完成任务 ====================

/// 完成任务（推荐）
pub async fn finish_item_with_store(
    item: Arc<ItemModel>,
    checked: bool,
    complete_sub_items: bool,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.complete_item(&item.id, checked, complete_sub_items).await
}

// ==================== 置顶任务 ====================

/// 使用全局 Store pin item（推荐）
pub async fn pin_item_with_store(
    item: Arc<ItemModel>,
    pinned: bool,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.update_item_pin(&item.id, pinned).await
}

// ==================== 按项目查询 ====================

/// 使用全局 Store 获取 tasks by project_id（推荐）
pub async fn get_items_by_project_id_with_store(
    project_id: &str,
    store: Arc<Store>,
) -> Result<Vec<ItemModel>, TodoError> {
    store.get_items_by_project(project_id).await
}

// ==================== 批量操作 ====================

/// 批量更新任务（推荐）
pub async fn batch_update_items_with_store(
    items: Vec<ItemModel>,
    store: Arc<Store>,
) -> Result<Vec<ItemModel>, TodoError> {
    store.batch_update_items(items).await
}
