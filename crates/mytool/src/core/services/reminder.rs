use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ReminderModel, error::TodoError};

/// 加载项目的提醒列表
pub async fn load_reminders_by_item(item_id: &str, db: DatabaseConnection) -> Vec<ReminderModel> {
    Store::new(db).await.unwrap().get_reminders_by_item(item_id).await.unwrap_or_default()
}

/// 使用全局 Store 加载 reminders by item（推荐）
pub async fn load_reminders_by_item_with_store(
    item_id: &str,
    store: Arc<Store>,
) -> Vec<ReminderModel> {
    store.get_reminders_by_item(item_id).await.unwrap_or_default()
}

/// 添加提醒
pub async fn add_reminder(
    reminder: ReminderModel,
    db: DatabaseConnection,
) -> Result<ReminderModel, TodoError> {
    Store::new(db).await?.insert_reminder(reminder).await
}

/// 使用全局 Store 添加 reminder（推荐）
pub async fn add_reminder_with_store(
    reminder: ReminderModel,
    store: Arc<Store>,
) -> Result<ReminderModel, TodoError> {
    store.insert_reminder(reminder).await
}

/// 删除提醒
pub async fn delete_reminder(reminder_id: &str, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).await?.delete_reminder(reminder_id).await
}

/// 使用全局 Store 删除 reminder（推荐）
pub async fn delete_reminder_with_store(
    reminder_id: &str,
    store: Arc<Store>,
) -> Result<u64, TodoError> {
    store.delete_reminder(reminder_id).await
}
