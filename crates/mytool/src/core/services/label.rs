use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::LabelModel, error::TodoError};

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).await.unwrap().get_all_labels().await.unwrap_or_default()
}
pub async fn add_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).await?.insert_label(label.as_ref().clone()).await
}

pub async fn mod_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).await?.update_label(label.as_ref().clone()).await
}

pub async fn del_label(label: Arc<LabelModel>, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).await?.delete_label(&label.id).await
}
