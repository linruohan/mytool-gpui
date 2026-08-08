//! Reminder service for business logic
//!
//! This module provides business logic for Reminder operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ReminderActiveModel, ReminderModel, prelude::*, reminders},
    error::TodoError,
};

/// Service for Reminder business operations
#[derive(Clone, Debug)]
pub struct ReminderService {
    db: Arc<DatabaseConnection>,
}

impl ReminderService {
    /// Create a new ReminderService
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
        active_reminder.insert(&*self.db).await.map_err(TodoError::from)
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, id: &str) -> Result<u64, TodoError> {
        let result = ReminderEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(result.rows_affected)
    }
}
