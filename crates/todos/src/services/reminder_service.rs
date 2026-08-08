//! Reminder service for business logic
//!
//! This module provides business logic for Reminder operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ReminderActiveModel, ReminderModel, prelude::*, reminders},
    error::TodoError,
    services::EventBus,
};

/// Service for Reminder business operations
#[derive(Clone, Debug)]
pub struct ReminderService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
}

impl ReminderService {
    /// Create a new ReminderService
    pub fn new(db: Arc<DatabaseConnection>, event_bus: Arc<EventBus>) -> Self {
        Self { db, event_bus }
    }

    /// Get reminders by item ID
    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await?;
        Ok(reminders)
    }

    /// Insert a new reminder
    pub async fn insert_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        let active_reminder: ReminderActiveModel = reminder.into();
        let reminder_model = active_reminder.insert(&*self.db).await?;

        let reminder_id = reminder_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::ReminderCreated(reminder_id));

        Ok(reminder_model)
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, id: &str) -> Result<u64, TodoError> {
        let id_clone = id.to_string();

        let result = ReminderEntity::delete_by_id(id).exec(&*self.db).await?;
        self.event_bus.publish(crate::services::event_bus::Event::ReminderDeleted(id_clone));

        Ok(result.rows_affected)
    }
}
