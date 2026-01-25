use sea_orm::DatabaseConnection;
use todos::{Store, entity::AttachmentModel, error::TodoError};

pub async fn load_attachments_by_item(
    item_id: &str,
    db: DatabaseConnection,
) -> Vec<AttachmentModel> {
    Store::new(db).await.get_attachments_by_itemid(item_id).await
}

pub async fn add_attachment(
    attachment: AttachmentModel,
    db: DatabaseConnection,
) -> Result<AttachmentModel, TodoError> {
    Store::new(db).await.insert_attachment(attachment).await
}

pub async fn delete_attachment(
    attachment_id: &str,
    db: DatabaseConnection,
) -> Result<u64, TodoError> {
    Store::new(db).await.delete_attachment(attachment_id).await
}
