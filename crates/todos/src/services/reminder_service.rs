//! Reminder service for business logic
//!
//! This module provides business logic for Reminder operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

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
        if let Some(item_id) = reminder.item_id.as_deref() {
            crate::utils::wait_for_item(&self.db, item_id).await?;
        }
        let active_reminder: ReminderActiveModel = reminder.into();
        active_reminder.insert(&*self.db).await.map_err(TodoError::from)
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, id: &str) -> Result<u64, TodoError> {
        let result = ReminderEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(result.rows_affected)
    }

    /// 未删除的提醒
    pub async fn get_active_reminders(&self) -> Result<Vec<ReminderModel>, TodoError> {
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await?;
        Ok(reminders)
    }

    /// 标记提醒已触发（软删除，避免重复弹出）
    pub async fn mark_reminder_notified(&self, id: &str) -> Result<(), TodoError> {
        let Some(model) = ReminderEntity::find_by_id(id).one(&*self.db).await? else {
            return Ok(());
        };
        let mut active: ReminderActiveModel = model.into();
        active.is_deleted = Set(true);
        active.update(&*self.db).await?;
        Ok(())
    }
}
