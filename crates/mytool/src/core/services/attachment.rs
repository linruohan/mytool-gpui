use sea_orm::DatabaseConnection;

use todos::{entity::AttachmentModel, error::TodoError};

/// 加载项目的附件列表
/// 注意：附件功能暂未实现，返回空向量
pub async fn load_attachments_by_item(
    _item_id: &str,
    _db: DatabaseConnection,
) -> Vec<AttachmentModel> {
    vec![]
}

/// 添加附件
/// 注意：附件功能暂未实现
pub async fn add_attachment(
    attachment: AttachmentModel,
    _db: DatabaseConnection,
) -> Result<AttachmentModel, TodoError> {
    Ok(attachment)
}

/// 删除附件
/// 注意：附件功能暂未实现
pub async fn delete_attachment(
    _attachment_id: &str,
    _db: DatabaseConnection,
) -> Result<u64, TodoError> {
    Ok(0)
}
