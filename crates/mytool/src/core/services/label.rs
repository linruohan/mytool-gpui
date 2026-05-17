use std::sync::Arc;

use todos::{Store, entity::LabelModel, error::TodoError};

// ==================== 加载标签 ====================

/// 使用全局 Store 加载 labels（推荐）
pub async fn load_labels_with_store(store: Arc<Store>) -> Result<Vec<LabelModel>, TodoError> {
    store.get_all_labels().await
}

// ==================== 添加标签 ====================

/// 添加标签（推荐）
pub async fn add_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<LabelModel, TodoError> {
    store.insert_label(label.as_ref().clone()).await
}

// ==================== 修改标签 ====================

/// 修改标签（推荐）
pub async fn mod_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<LabelModel, TodoError> {
    store.update_label(label.as_ref().clone()).await
}

// ==================== 删除标签 ====================

/// 删除标签（推荐）
pub async fn del_label_with_store(
    label: Arc<LabelModel>,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_label(&label.id).await
}
