use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::LabelModel, error::TodoError};

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).get_all_labels().await.unwrap_or_default()
}
pub async fn add_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).insert_label(label.as_ref().clone()).await
}

pub async fn mod_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).update_label(label.as_ref().clone()).await
}

pub async fn del_label(label: Arc<LabelModel>, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).delete_label(&label.id).await
}
#[allow(unused)]
pub async fn get_label_by_id(label_id: &str, db: DatabaseConnection) -> Option<LabelModel> {
    Store::new(db).get_label(label_id).await
}

/// 获取 Item 的所有 Labels（通过 item_labels 关联表）
pub async fn get_labels_by_item(item_id: &str, db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).get_labels_by_item(item_id).await.unwrap_or_default()
}

/// 为 Item 添加 Label
pub async fn add_label_to_item(
    item_id: &str,
    label_name: &str,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).add_label_to_item(item_id, label_name).await
}

/// 从 Item 移除 Label
pub async fn remove_label_from_item(
    item_id: &str,
    label_id: &str,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).remove_label_from_item(item_id, label_id).await
}
