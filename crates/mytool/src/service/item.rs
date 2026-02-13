use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel},
    error::TodoError,
};

pub async fn load_items(_db: DatabaseConnection) -> Vec<ItemModel> {
    // 暂时返回空向量，实际实现需要根据业务逻辑获取所有项目
    vec![]
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
