use sea_orm::DatabaseConnection;
use todos::{Store, entity::ReminderModel, error::TodoError};

pub async fn load_reminders_by_item(item_id: &str, db: DatabaseConnection) -> Vec<ReminderModel> {
    Store::new(db).get_reminders_by_item(item_id).await.unwrap_or_default()
}

pub async fn add_reminder(
    reminder: ReminderModel,
    db: DatabaseConnection,
) -> Result<ReminderModel, TodoError> {
    Store::new(db).insert_reminder(reminder).await
}

pub async fn delete_reminder(reminder_id: &str, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).delete_reminder(reminder_id).await
}
