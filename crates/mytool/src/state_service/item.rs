use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel, error::TodoError};

/// 获取所有未完成的任务项
/// 注意：这是获取所有任务的主要入口，其他视图通过过滤此数据获得子集
pub async fn load_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_incomplete_items().await.unwrap_or_default()
}
pub async fn add_item(
    item: Arc<ItemModel>,
    db: DatabaseConnection,
) -> Result<ItemModel, TodoError> {
    Store::new(db).insert_item(item.as_ref().clone(), true).await
}

pub async fn mod_item(
    item: Arc<ItemModel>,
    db: DatabaseConnection,
) -> Result<ItemModel, TodoError> {
    Store::new(db).update_item(item.as_ref().clone(), "").await
}

pub async fn del_item(item: Arc<ItemModel>, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).delete_item(&item.id).await
}

// 修改item完成状态
pub async fn finish_item(
    item: Arc<ItemModel>,
    checked: bool,
    complete_sub_items: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).complete_item(&item.id, checked, complete_sub_items).await
}
pub async fn pin_item(
    item: Arc<ItemModel>,
    pinned: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).update_item_pin(&item.id, pinned).await
}
pub async fn get_items_by_project_id(project_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_items_by_project(project_id).await.unwrap_or_default()
}
