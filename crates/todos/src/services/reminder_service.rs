//! Reminder service for business logic
//!
//! This module provides business logic for Reminder operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{ReminderActiveModel, ReminderModel, prelude::*, reminders},
    error::TodoError,
    repositories::{ReminderRepository, ReminderRepositoryImpl},
    services::{EventBus, MetricsCollector},
};

/// Service for Reminder business operations
#[derive(Clone, Debug)]
pub struct ReminderService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    reminder_repo: ReminderRepositoryImpl,
}

impl ReminderService {
    /// Create a new ReminderService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let reminder_repo = ReminderRepositoryImpl::new(db.clone());
        Self { db, event_bus, metrics, reminder_repo }
    }

    /// Get a reminder by ID
    pub async fn get_reminder(&self, id: &str) -> Option<ReminderModel> {
        let result: Result<ReminderModel, TodoError> = self.reminder_repo.find_by_id(id).await;
        result.ok()
    }

    /// Get all reminders
    pub async fn get_all_reminders(&self) -> Result<Vec<ReminderModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_reminders");
        let reminders = ReminderEntity::find().all(&*self.db).await?;
        self.metrics.record_operation("get_all_reminders", reminders.len()).await;
        Ok(reminders)
    }

    /// Get reminders by item ID
    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_reminders_by_item");
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_reminders_by_item", reminders.len()).await;
        Ok(reminders)
    }

    /// Insert a new reminder
    pub async fn insert_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_reminder");
        let mut active_reminder: ReminderActiveModel = reminder.into();
        let reminder_model = active_reminder.insert(&*self.db).await?;

        let reminder_id = reminder_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::ReminderCreated(reminder_id));

        self.metrics.record_operation("insert_reminder", 1).await;
        Ok(reminder_model)
    }

    /// Update an existing reminder
    pub async fn update_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        let _timer = self.metrics.start_timer("update_reminder");
        let reminder_id = reminder.id.clone();
        let mut active_reminder: ReminderActiveModel = reminder.into();
        let result = active_reminder.update(&*self.db).await?;

        self.event_bus.publish(crate::services::event_bus::Event::ReminderUpdated(reminder_id));

        self.metrics.record_operation("update_reminder", 1).await;
        Ok(result)
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, id: &str) -> Result<u64, TodoError> {
        let _timer = self.metrics.start_timer("delete_reminder");
        let id_clone = id.to_string();

        let result = ReminderEntity::delete_by_id(id).exec(&*self.db).await?;
        self.event_bus.publish(crate::services::event_bus::Event::ReminderDeleted(id_clone));

        self.metrics.record_operation("delete_reminder", 1).await;
        Ok(result.rows_affected)
    }

    // ==================== Additional Business Logic Methods ====================

    /// Get reminders due before a specific time
    pub async fn get_reminders_due_before(
        &self,
        due_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_reminders_due_before");
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::Due.eq(due_time.to_string()))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_reminders_due_before", reminders.len()).await;
        Ok(reminders)
    }

    /// Get reminders due after a specific time
    pub async fn get_reminders_due_after(
        &self,
        due_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_reminders_due_after");
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::Due.eq(due_time.to_string()))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_reminders_due_after", reminders.len()).await;
        Ok(reminders)
    }

    /// Get reminders in a time range
    pub async fn get_reminders_in_range(
        &self,
        start_time: &chrono::NaiveDateTime,
        end_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_reminders_in_range");
        let reminders = ReminderEntity::find()
            .filter(reminders::Column::Due.eq(start_time.to_string()))
            .filter(reminders::Column::Due.eq(end_time.to_string()))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_reminders_in_range", reminders.len()).await;
        Ok(reminders)
    }
}
