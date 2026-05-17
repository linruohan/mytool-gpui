use std::sync::Arc;

use todos::{Store, entity::ReminderModel, error::TodoError};

/// 使用全局 Store 加载 reminders by item（推荐）
pub async fn load_reminders_by_item_with_store(
    item_id: &str,
    store: Arc<Store>,
) -> Vec<ReminderModel> {
    store.get_reminders_by_item(item_id).await.unwrap_or_default()
}

/// 添加提醒（推荐）
pub async fn add_reminder_with_store(
    reminder: ReminderModel,
    store: Arc<Store>,
) -> Result<ReminderModel, TodoError> {
    store.insert_reminder(reminder).await
}

/// 删除提醒（推荐）
pub async fn delete_reminder_with_store(
    reminder_id: &str,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_reminder(reminder_id).await
}
