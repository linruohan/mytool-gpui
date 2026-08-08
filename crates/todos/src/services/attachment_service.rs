//! Attachment service for business logic
//!
//! This module provides business logic for Attachment operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{AttachmentActiveModel, AttachmentModel, attachments, prelude::*},
    error::TodoError,
    services::{EventBus, MetricsCollector},
};

/// Service for Attachment business operations
#[derive(Clone, Debug)]
pub struct AttachmentService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
}

impl AttachmentService {
    /// Create a new AttachmentService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self { db, event_bus, metrics }
    }

    /// Get attachments by item ID
    pub async fn get_attachments_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<AttachmentModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_attachments_by_item");
        let attachments = AttachmentEntity::find()
            .filter(attachments::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_attachments_by_item", attachments.len()).await;
        Ok(attachments)
    }

    /// Insert a new attachment
    pub async fn insert_attachment(
        &self,
        attachment: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_attachment");
        let active_attachment: AttachmentActiveModel = attachment.into();
        let attachment_model = active_attachment.insert(&*self.db).await?;

        let attachment_id = attachment_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::AttachmentCreated(attachment_id));

        self.metrics.record_operation("insert_attachment", 1).await;
        Ok(attachment_model)
    }

    /// Delete an attachment
    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        let _timer = self.metrics.start_timer("delete_attachment");
        let id_clone = id.to_string();

        let result = AttachmentEntity::delete_by_id(id).exec(&*self.db).await?;
        self.event_bus.publish(crate::services::event_bus::Event::AttachmentDeleted(id_clone));

        self.metrics.record_operation("delete_attachment", 1).await;
        Ok(result.rows_affected)
    }
}
