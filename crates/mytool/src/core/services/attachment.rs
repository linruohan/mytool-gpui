use sea_orm::DatabaseConnection;
use todos::{entity::AttachmentModel, error::TodoError};

pub async fn load_attachments_by_item(
    _item_id: &str,
    _db: DatabaseConnection,
) -> Vec<AttachmentModel> {
    // 附件功能暂未实现
    vec![]
}

pub async fn add_attachment(
    attachment: AttachmentModel,
    _db: DatabaseConnection,
) -> Result<AttachmentModel, TodoError> {
    // 附件功能暂未实现
    Ok(attachment)
}

pub async fn delete_attachment(
    _attachment_id: &str,
    _db: DatabaseConnection,
) -> Result<u64, TodoError> {
    // 附件功能暂未实现
    Ok(0)
}
