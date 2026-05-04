use sea_orm::DatabaseConnection;
use todos::{Store, entity::AttachmentModel, error::TodoError};

/// 加载项目的附件列表
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

/// 添加附件
pub async fn add_attachment(
    attachment: AttachmentModel,
    db: DatabaseConnection,
) -> Result<AttachmentModel, TodoError> {
    let store = Store::new(db).await?;
    store.insert_attachment(attachment).await
}

/// 删除附件
pub async fn delete_attachment(
    attachment_id: &str,
    db: DatabaseConnection,
) -> Result<u64, TodoError> {
    let store = Store::new(db).await?;
    store.delete_attachment(attachment_id).await
}
