use std::sync::Arc;

use todos::{Store, entity::AttachmentModel, error::TodoError};

/// 使用全局 Store 加载项目的附件列表（推荐）
pub async fn load_attachments_by_item_with_store(
    item_id: &str,
    store: Arc<Store>,
) -> Vec<AttachmentModel> {
    match store.get_attachments_by_item(item_id).await {
        Ok(attachments) => attachments,
        Err(e) => {
            tracing::error!("Failed to load attachments for item {}: {:?}", item_id, e);
            vec![]
        },
    }
}

/// 添加附件（推荐）
pub async fn add_attachment_with_store(
    attachment: AttachmentModel,
    store: Arc<Store>,
) -> Result<AttachmentModel, TodoError> {
    store.insert_attachment(attachment).await
}

/// 删除附件（推荐）
pub async fn delete_attachment_with_store(
    attachment_id: &str,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_attachment(attachment_id).await
}
