//! Attachment service for business logic
//!
//! This module provides business logic for Attachment operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{AttachmentActiveModel, AttachmentModel, attachments, prelude::*},
    error::TodoError,
    repositories::{AttachmentRepository, AttachmentRepositoryImpl},
    services::{EventBus, MetricsCollector},
};

/// Service for Attachment business operations
#[derive(Clone, Debug)]
pub struct AttachmentService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    attachment_repo: AttachmentRepositoryImpl,
}

impl AttachmentService {
    /// Create a new AttachmentService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let attachment_repo = AttachmentRepositoryImpl::new(db.clone());
        Self { db, event_bus, metrics, attachment_repo }
    }

    /// Get an attachment by ID
    pub async fn get_attachment(&self, id: &str) -> Option<AttachmentModel> {
        let result: Result<AttachmentModel, TodoError> = self.attachment_repo.find_by_id(id).await;
        result.ok()
    }

    /// Get all attachments
    pub async fn get_all_attachments(&self) -> Result<Vec<AttachmentModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_attachments");
        let attachments = AttachmentEntity::find().all(&*self.db).await?;
        self.metrics.record_operation("get_all_attachments", attachments.len()).await;
        Ok(attachments)
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
        let mut active_attachment: AttachmentActiveModel = attachment.into();
        let attachment_model = active_attachment.insert(&*self.db).await?;

        let attachment_id = attachment_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::AttachmentCreated(attachment_id));

        self.metrics.record_operation("insert_attachment", 1).await;
        Ok(attachment_model)
    }

    /// Update an existing attachment
    pub async fn update_attachment(
        &self,
        attachment: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        let _timer = self.metrics.start_timer("update_attachment");
        let attachment_id = attachment.id.clone();

        let active_attachment = AttachmentActiveModel {
            id: Set(attachment.id),
            item_id: Set(attachment.item_id),
            file_type: Set(attachment.file_type),
            file_name: Set(attachment.file_name),
            file_size: Set(attachment.file_size),
            file_path: Set(attachment.file_path),
        };

        let result = active_attachment.update(&*self.db).await?;

        self.event_bus.publish(crate::services::event_bus::Event::AttachmentUpdated(attachment_id));

        self.metrics.record_operation("update_attachment", 1).await;
        Ok(result)
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
