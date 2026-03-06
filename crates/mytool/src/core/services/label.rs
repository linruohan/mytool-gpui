use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::LabelModel, error::TodoError};

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).await.unwrap().get_all_labels().await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 加载 labels
pub async fn load_labels_with_store(store: Arc<Store>) -> Vec<LabelModel> {
    store.get_all_labels().await.unwrap_or_default()
}

pub async fn add_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).await?.insert_label(label.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 添加 label
pub async fn add_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<LabelModel, TodoError> {
    store.insert_label(label.as_ref().clone()).await
}

pub async fn mod_label(
    label: Arc<LabelModel>,
    db: DatabaseConnection,
) -> Result<LabelModel, TodoError> {
    Store::new(db).await?.update_label(label.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 修改 label
pub async fn mod_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<LabelModel, TodoError> {
    store.update_label(label.as_ref().clone()).await
}

pub async fn del_label(label: Arc<LabelModel>, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).await?.delete_label(&label.id).await
}

/// 🚀 新增：使用全局 Store 删除 label
pub async fn del_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_label(&label.id).await
}
