use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel, error::TodoError};

/// 获取所有任务项（包括已完成和未完成的）
/// 注意：这是获取所有任务的主要入口，其他视图通过过滤此数据获得子集
pub async fn load_items(db: DatabaseConnection) -> Vec<ItemModel> {
    // 🚀 优化：使用传入的 db 创建临时 Store，后续会改为使用全局 Store
    Store::new(db).await.unwrap().get_all_items().await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 加载 items
pub async fn load_items_with_store(store: Arc<Store>) -> Vec<ItemModel> {
    store.get_all_items().await.unwrap_or_default()
}

pub async fn add_item(
    item: Arc<ItemModel>,
    db: DatabaseConnection,
) -> Result<ItemModel, TodoError> {
    Store::new(db).await?.insert_item(item.as_ref().clone(), true).await
}

/// 🚀 新增：使用全局 Store 添加 item
pub async fn add_item_with_store(
    item: Arc<ItemModel>,
    store: Arc<Store>,
) -> Result<ItemModel, TodoError> {
    store.insert_item(item.as_ref().clone(), true).await
}

pub async fn mod_item(
    item: Arc<ItemModel>,
    db: DatabaseConnection,
) -> Result<ItemModel, TodoError> {
    Store::new(db).await?.update_item(item.as_ref().clone(), "").await
}

/// 🚀 新增：使用全局 Store 更新 item（避免重复创建 Store）
pub async fn mod_item_with_store(
    item: Arc<ItemModel>,
    store: Arc<Store>,
) -> Result<ItemModel, TodoError> {
    store.update_item(item.as_ref().clone(), "").await
}

pub async fn del_item(item: Arc<ItemModel>, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).await?.delete_item(&item.id).await
}

/// 🚀 新增：使用全局 Store 删除 item
pub async fn del_item_with_store(item: Arc<ItemModel>, store: Arc<Store>) -> Result<(), TodoError> {
    store.delete_item(&item.id).await
}

// 修改 item 完成状态
pub async fn finish_item(
    item: Arc<ItemModel>,
    checked: bool,
    complete_sub_items: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await?.complete_item(&item.id, checked, complete_sub_items).await
}

/// 🚀 新增：使用全局 Store 完成 item
pub async fn finish_item_with_store(
    item: Arc<ItemModel>,
    checked: bool,
    complete_sub_items: bool,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.complete_item(&item.id, checked, complete_sub_items).await
}

pub async fn pin_item(
    item: Arc<ItemModel>,
    pinned: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await?.update_item_pin(&item.id, pinned).await
}

/// 🚀 新增：使用全局 Store pin item
pub async fn pin_item_with_store(
    item: Arc<ItemModel>,
    pinned: bool,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.update_item_pin(&item.id, pinned).await
}

pub async fn get_items_by_project_id(project_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.unwrap().get_items_by_project(project_id).await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 获取 items by project_id
pub async fn get_items_by_project_id_with_store(
    project_id: &str,
    store: Arc<Store>,
) -> Vec<ItemModel> {
    store.get_items_by_project(project_id).await.unwrap_or_default()
}

// ==================== 批量操作 ====================

/// 批量添加任务
pub async fn batch_add_items(
    items: Vec<ItemModel>,
    db: DatabaseConnection,
) -> Result<Vec<ItemModel>, TodoError> {
    Store::new(db).await?.batch_insert_items(items).await
}

/// 批量更新任务
pub async fn batch_update_items(
    items: Vec<ItemModel>,
    db: DatabaseConnection,
) -> Result<Vec<ItemModel>, TodoError> {
    Store::new(db).await?.batch_update_items(items).await
}

/// 批量删除任务
pub async fn batch_delete_items(
    item_ids: Vec<String>,
    db: DatabaseConnection,
) -> Result<usize, TodoError> {
    Store::new(db).await?.batch_delete_items(item_ids).await
}

/// 批量完成/取消完成任务
pub async fn batch_complete_items(
    item_ids: Vec<String>,
    checked: bool,
    complete_sub_items: bool,
    db: DatabaseConnection,
) -> Result<usize, TodoError> {
    Store::new(db).await?.batch_complete_items(item_ids, checked, complete_sub_items).await
}
