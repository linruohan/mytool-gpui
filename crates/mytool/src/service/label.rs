use std::rc::Rc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::LabelModel, error::TodoError};

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).get_all_labels().await.unwrap_or_default()
}
pub async fn add_label(
    label: Rc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).insert_label(label.as_ref().clone()).await
}

pub async fn mod_label(
    label: Rc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).update_label(label.as_ref().clone()).await
}

pub async fn del_label(label: Rc<LabelModel>, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).delete_label(&label.id).await
}
#[allow(unused)]
pub async fn get_label_by_id(label_id: &str, db: DatabaseConnection) -> Option<LabelModel> {
    Store::new(db).get_label(label_id).await
}
