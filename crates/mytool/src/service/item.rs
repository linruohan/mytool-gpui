use std::rc::Rc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel},
    error::TodoError,
};

pub async fn load_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.items().await
}
pub async fn add_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<ItemModel, TodoError> {
    Store::new(db).await.insert_item(item.as_ref().clone(), true).await
}

pub async fn mod_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<ItemModel, TodoError> {
    Store::new(db).await.update_item(item.as_ref().clone(), "").await
}

pub async fn del_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).await.delete_item(&item.id).await
}
// 修改item完成状态
pub async fn finish_item(
    item: Rc<ItemModel>,
    checked: bool,
    complete_sub_items: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await.complete_item(&item.id, checked, complete_sub_items).await
}
pub async fn pin_item(
    item: Rc<ItemModel>,
    pinned: bool,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await.update_item_pin(&item.id, pinned).await
}
#[allow(unused)]
pub async fn get_project_items(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_project(&project.id).await
}
pub async fn get_items_by_project_id(project_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_project(project_id).await
}
pub async fn get_items_completed(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.get_items_completed().await
}
pub async fn get_items_today(db: DatabaseConnection) -> Vec<ItemModel> {
    let today = chrono::Local::now().naive_local();
    Store::new(db).await.get_items_by_date(&today, false).await
}
pub async fn get_items_pinned(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.get_items_pinned(false).await
}
pub async fn get_items_scheduled(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_scheduled(false).await
}
