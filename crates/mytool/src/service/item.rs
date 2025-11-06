use sea_orm::DatabaseConnection;
use std::rc::Rc;
use todos::entity::{ItemModel, ProjectModel};
use todos::error::TodoError;
use todos::Store;

pub async fn load_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.items().await
}
pub async fn add_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<ItemModel, TodoError> {
    Store::new(db)
        .await
        .insert_item(item.as_ref().clone(), true)
        .await
}

pub async fn mod_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<ItemModel, TodoError> {
    Store::new(db)
        .await
        .update_item(item.as_ref().clone(), "")
        .await
}

pub async fn del_item(item: Rc<ItemModel>, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).await.delete_item(&item.id).await
}

pub async fn get_project_items(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_project(&project.id).await
}
pub async fn get_items_by_project_id(
    project_id: &str,
    db: DatabaseConnection,
) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_project(project_id).await
}
