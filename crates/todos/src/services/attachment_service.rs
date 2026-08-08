//! Attachment service for business logic
//!
//! This module provides business logic for Attachment operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{AttachmentActiveModel, AttachmentModel, attachments, prelude::*},
    error::TodoError,
};

/// Service for Attachment business operations
#[derive(Clone, Debug)]
pub struct AttachmentService {
    db: Arc<DatabaseConnection>,
}

impl AttachmentService {
    /// Create a new AttachmentService
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get attachments by item ID
    pub async fn get_attachments_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<AttachmentModel>, TodoError> {
        let attachments = AttachmentEntity::find()
            .filter(attachments::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await?;
        Ok(attachments)
    }

    /// Insert a new attachment
    pub async fn insert_attachment(
        &self,
        attachment: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        let active_attachment: AttachmentActiveModel = attachment.into();
        active_attachment.insert(&*self.db).await.map_err(TodoError::from)
    }

    /// Delete an attachment
    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        let result = AttachmentEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(result.rows_affected)
    }
}
