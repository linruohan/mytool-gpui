use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::AttachmentModel, error::TodoError};

/// 加载项目的附件列表
/// 注意：附件功能暂未实现，返回空向量
pub async fn load_attachments_by_item(
    _item_id: &str,
    _db: DatabaseConnection,
) -> Vec<AttachmentModel> {
    vec![]
}

/// 使用全局 Store 加载附件（推荐）
#[allow(dead_code)]
pub async fn load_attachments_by_item_with_store(
    _item_id: &str,
    _store: Arc<Store>,
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

/// 使用全局 Store 添加附件（推荐）
#[allow(dead_code)]
pub async fn add_attachment_with_store(
    attachment: AttachmentModel,
    _store: Arc<Store>,
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

/// 使用全局 Store 删除附件（推荐）
#[allow(dead_code)]
pub async fn delete_attachment_with_store(
    _attachment_id: &str,
    _store: Arc<Store>,
) -> Result<u64, TodoError> {
    Ok(0)
}
