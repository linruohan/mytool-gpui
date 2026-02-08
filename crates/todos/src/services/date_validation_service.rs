//! Date Validation Service
//!
//! This service provides date validation operations for items.
//! It checks if items match specific date criteria.

use std::sync::Arc;

use chrono::Datelike;
use sea_orm::DatabaseConnection;

use crate::{
    entity::{ItemModel, items::Entity as ItemEntity},
    error::TodoError,
    objects::item::Item,
    utils::DateTime,
};

/// Date Validation Service
#[derive(Clone, Debug)]
pub struct DateValidationService {
    db: Arc<DatabaseConnection>,
}

impl DateValidationService {
    /// Create a new DateValidationService
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get the database connection
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Validate if an item matches a specific date
    pub async fn valid_item_by_date(
        &self,
        item_id: &str,
        date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item): Result<crate::objects::item::Item, TodoError> =
            Item::from_db(self.db().clone(), &item_model.id).await
        else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        let date_util = DateTime::default();
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| date_util.is_same_day(&due, date))
            .unwrap_or(false)
    }

    /// Validate if an item matches a date range
    pub async fn valid_item_by_date_range(
        &self,
        item_id: &str,
        start_date: &chrono::NaiveDateTime,
        end_date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item): Result<crate::objects::item::Item, TodoError> =
            Item::from_db(self.db().clone(), &item_model.id).await
        else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due >= *start_date && due <= *end_date)
            .unwrap_or(false)
    }

    /// Validate if an item matches a specific month
    pub async fn valid_item_by_month(
        &self,
        item_id: &str,
        date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item): Result<crate::objects::item::Item, TodoError> =
            Item::from_db(self.db().clone(), &item_model.id).await
        else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due.year() == date.year() && due.month() == date.month())
            .unwrap_or(false)
    }

    /// Validate if an item is overdue
    pub async fn valid_item_by_overdue(&self, item_id: &str, checked: bool) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item): Result<crate::objects::item::Item, TodoError> =
            Item::from_db(self.db().clone(), &item_model.id).await
        else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        let now = chrono::Utc::now().naive_utc();
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due < now && DateTime::default().is_same_day(&due, &now))
            .unwrap_or(false)
    }

    /// Get an item by ID
    async fn get_item(&self, id: &str) -> Option<ItemModel> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let result = ItemEntity::find_by_id(id.to_string()).one(self.db()).await.ok().flatten();
        result
    }
}
