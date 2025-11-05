use sea_orm::DatabaseConnection;
use std::rc::Rc;
use todos::entity::LabelModel;
use todos::error::TodoError;
use todos::Store;

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).await.labels().await
}
pub async fn add_label(
    label: Rc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db)
        .await
        .insert_label(label.as_ref().clone())
        .await
}

pub async fn mod_label(
    label: Rc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db)
        .await
        .update_label(label.as_ref().clone())
        .await
}

pub async fn del_label(label: Rc<LabelModel>, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).await.delete_label(&label.id).await
}
