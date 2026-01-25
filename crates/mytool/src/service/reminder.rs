use sea_orm::DatabaseConnection;
use todos::{Store, entity::ReminderModel, error::TodoError};

pub async fn load_reminders_by_item(item_id: &str, db: DatabaseConnection) -> Vec<ReminderModel> {
    Store::new(db).await.get_reminders_by_item(item_id).await
}

pub async fn add_reminder(
    reminder: ReminderModel,
    db: DatabaseConnection,
) -> Result<ReminderModel, TodoError> {
    Store::new(db).await.insert_reminder(reminder).await
}

pub async fn delete_reminder(reminder_id: &str, db: DatabaseConnection) -> Result<u64, TodoError> {
    Store::new(db).await.delete_reminder(reminder_id).await
}
