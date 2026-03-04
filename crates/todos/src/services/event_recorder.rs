use std::sync::Arc;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

#[derive(Debug)]
pub struct EventRecorder {
    db: Arc<DatabaseConnection>,
}

impl EventRecorder {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn record_event(
        &self,
        event_type: &str,
        object_id: &str,
        object_type: &str,
        object_key: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<(), DbErr> {
        let sql = r#"
            INSERT INTO OEvents (
                event_type,
                object_id,
                object_type,
                object_key,
                object_old_value,
                object_new_value
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#;

        self.db
            .execute(Statement::from_sql_and_values(sea_orm::DbBackend::Sqlite, sql, vec![
                event_type.into(),
                object_id.into(),
                object_type.into(),
                object_key.into(),
                old_value.unwrap_or("").into(),
                new_value.unwrap_or("").into(),
            ]))
            .await?;

        Ok(())
    }

    pub async fn get_events_by_object(
        &self,
        object_type: &str,
        object_id: &str,
    ) -> Result<Vec<serde_json::Value>, DbErr> {
        let sql = r#"
            SELECT * FROM OEvents 
            WHERE object_type = ? AND object_id = ? 
            ORDER BY event_date DESC
        "#;

        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(sea_orm::DbBackend::Sqlite, sql, vec![
                object_type.into(),
                object_id.into(),
            ]))
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let event = serde_json::json!(
                {
                    "id": row.try_get::<i32>("", "id")?,
                    "event_type": row.try_get::<String>("", "event_type")?,
                    "event_date": row.try_get::<String>("", "event_date")?,
                    "object_id": row.try_get::<String>("", "object_id")?,
                    "object_type": row.try_get::<String>("", "object_type")?,
                    "object_key": row.try_get::<String>("", "object_key")?,
                    "object_old_value": row.try_get::<String>("", "object_old_value")?,
                    "object_new_value": row.try_get::<String>("", "object_new_value")?,
                }
            );
            events.push(event);
        }

        Ok(events)
    }

    pub async fn cleanup_old_events(&self, days_to_keep: i32) -> Result<(), DbErr> {
        let sql = format!(
            "DELETE FROM OEvents WHERE event_date < datetime('now', '-{} days')",
            days_to_keep
        );

        self.db.execute(Statement::from_string(sea_orm::DbBackend::Sqlite, sql)).await?;
        Ok(())
    }
}
