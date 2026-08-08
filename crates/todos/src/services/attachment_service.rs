//! Attachment service for business logic
//!
//! This module provides business logic for Attachment operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{AttachmentActiveModel, AttachmentModel, attachments, prelude::*},
    error::TodoError,
    services::EventBus,
};

/// Service for Attachment business operations
#[derive(Clone, Debug)]
pub struct AttachmentService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
}

impl AttachmentService {
    /// Create a new AttachmentService
    pub fn new(db: Arc<DatabaseConnection>, event_bus: Arc<EventBus>) -> Self {
        Self { db, event_bus }
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
        let attachment_model = active_attachment.insert(&*self.db).await?;

        let attachment_id = attachment_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::AttachmentCreated(attachment_id));

        Ok(attachment_model)
    }

    /// Delete an attachment
    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        let id_clone = id.to_string();

        let result = AttachmentEntity::delete_by_id(id).exec(&*self.db).await?;
        self.event_bus.publish(crate::services::event_bus::Event::AttachmentDeleted(id_clone));

        Ok(result.rows_affected)
    }
}
