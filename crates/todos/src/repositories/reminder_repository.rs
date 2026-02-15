//! Reminder repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ReminderModel, prelude::ReminderEntity},
    error::TodoError,
};

/// Repository trait for Reminder operations
#[async_trait::async_trait]
pub trait ReminderRepository {
    async fn find_by_id(&self, id: &str) -> Result<ReminderModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ReminderModel>, TodoError>;
    async fn find_by_item(&self, item_id: &str) -> Result<Vec<ReminderModel>, TodoError>;
}

/// Implementation of ReminderRepository
#[derive(Clone, Debug)]
pub struct ReminderRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ReminderRepositoryImpl {
    /// Create a new ReminderRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ReminderRepository for ReminderRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ReminderModel, TodoError> {
        ReminderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
            .and_then(|reminder| {
                reminder.ok_or_else(|| TodoError::NotFound(format!("Reminder {} not found", id)))
            })
    }

    async fn find_all(&self) -> Result<Vec<ReminderModel>, TodoError> {
        ReminderEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_item(&self, item_id: &str) -> Result<Vec<ReminderModel>, TodoError> {
        ReminderEntity::find()
            .filter(crate::entity::reminders::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
