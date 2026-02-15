use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel},
    error::TodoError,
};

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

/// 根据ID删除任务（用于增量更新）
pub async fn del_item_by_id(item_id: &str, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).delete_item(item_id).await
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
#[allow(unused)]
pub async fn get_project_items(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<ItemModel> {
    Store::new(db).get_items_by_project(&project.id).await.unwrap_or_default()
}
pub async fn get_inbox_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_incomplete_items().await.unwrap_or_default()
}
pub async fn get_items_by_project_id(project_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_items_by_project(project_id).await.unwrap_or_default()
}
pub async fn get_items_completed(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_completed_items().await.unwrap_or_default()
}
pub async fn get_items_today(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_items_due_today().await.unwrap_or_default()
}
pub async fn get_items_pinned(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_pinned_items().await.unwrap_or_default()
}

#[allow(dead_code)]
pub async fn get_incomplete_pinned_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_incomplete_pinned_items().await.unwrap_or_default()
}
pub async fn get_items_scheduled(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_scheduled_items().await.unwrap_or_default()
}
