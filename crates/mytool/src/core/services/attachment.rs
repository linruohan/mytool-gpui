use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::AttachmentModel, error::TodoError};

/// 加载项目的附件列表
#[deprecated(since = "2.0.0", note = "请使用 load_attachments_by_item_with_store() 方法")]
pub async fn load_attachments_by_item(
    item_id: &str,
    db: DatabaseConnection,
) -> Vec<AttachmentModel> {
    match Store::new(db).await {
        Ok(store) => match store.get_attachments_by_item(item_id).await {
            Ok(attachments) => attachments,
            Err(e) => {
                tracing::error!("Failed to load attachments for item {}: {:?}", item_id, e);
                vec![]
            },
        },
        Err(e) => {
            tracing::error!("Failed to create store: {:?}", e);
            vec![]
        },
    }
}

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

/// 添加附件
#[deprecated(since = "2.0.0", note = "请使用 add_attachment_with_store() 方法")]
pub async fn add_attachment(
    attachment: AttachmentModel,
    db: DatabaseConnection,
) -> Result<AttachmentModel, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    add_attachment_with_store(attachment, store).await
}

/// 使用全局 Store 添加附件（推荐）
pub async fn add_attachment_with_store(
    attachment: AttachmentModel,
    store: Arc<Store>,
) -> Result<AttachmentModel, TodoError> {
    store.insert_attachment(attachment).await
}

/// 删除附件
#[deprecated(since = "2.0.0", note = "请使用 delete_attachment_with_store() 方法")]
pub async fn delete_attachment(
    attachment_id: &str,
    db: DatabaseConnection,
) -> Result<u64, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    delete_attachment_with_store(attachment_id, store).await
}

/// 使用全局 Store 删除附件（推荐）
pub async fn delete_attachment_with_store(
    attachment_id: &str,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_attachment(attachment_id).await
}
