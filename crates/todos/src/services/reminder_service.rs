//! Reminder service for business logic
//!
//! This module provides business logic for Reminder operations,
//! separating it from data access layer.

use std::sync::Arc;

use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{
    entity::{ReminderActiveModel, ReminderModel, prelude::*, reminders},
    error::TodoError,
};

fn schedule_models(rows: Vec<(String, Option<String>, Option<String>)>) -> Vec<ReminderModel> {
    rows.into_iter()
        .map(|(id, item_id, due)| ReminderModel { id, item_id, due, ..Default::default() })
        .collect()
}

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
            .filter(reminders::Column::IsDeleted.eq(false))
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

    /// 已到期的提醒，再加上下一条还没到期的，供轮询决定何时再醒。
    ///
    /// 时间按 `YYYY-MM-DD HH:MM:SS` 的文本顺序比较。只含日期的值会先被取回，
    /// 再由调用方按早上 9 点判断是不是真的到期。
    pub async fn get_schedule_reminders(
        &self,
        now: NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let due = ReminderEntity::find()
            .select_only()
            .column(reminders::Column::Id)
            .column(reminders::Column::ItemId)
            .column(reminders::Column::Due)
            .filter(reminders::Column::IsDeleted.eq(false))
            .filter(reminders::Column::Due.lte(&now_str))
            .into_tuple::<(String, Option<String>, Option<String>)>()
            .all(&*self.db)
            .await?;
        let next = ReminderEntity::find()
            .select_only()
            .column(reminders::Column::Id)
            .column(reminders::Column::ItemId)
            .column(reminders::Column::Due)
            .filter(reminders::Column::IsDeleted.eq(false))
            .filter(reminders::Column::Due.gt(&now_str))
            .order_by_asc(reminders::Column::Due)
            .limit(1)
            .into_tuple::<(String, Option<String>, Option<String>)>()
            .all(&*self.db)
            .await?;
        let mut rows = schedule_models(due);
        rows.extend(schedule_models(next));
        Ok(rows)
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

    /// 软删除后再次激活并改期（延后提醒）
    pub async fn reschedule_reminder(&self, id: &str, due: String) -> Result<(), TodoError> {
        let Some(model) = ReminderEntity::find_by_id(id).one(&*self.db).await? else {
            return Ok(());
        };
        let mut active: ReminderActiveModel = model.into();
        active.due = Set(Some(due));
        active.is_deleted = Set(false);
        active.update(&*self.db).await?;
        Ok(())
    }
}
