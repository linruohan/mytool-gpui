//! Attachment repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{AttachmentModel, prelude::AttachmentEntity},
    error::TodoError,
};

/// Repository trait for Attachment operations
#[async_trait]
pub trait AttachmentRepository {
    async fn find_by_id(&self, id: &str) -> Result<AttachmentModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<AttachmentModel>, TodoError>;
    async fn find_by_item(&self, item_id: &str) -> Result<Vec<AttachmentModel>, TodoError>;
}

/// Implementation of AttachmentRepository
#[derive(Clone, Debug)]
pub struct AttachmentRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl AttachmentRepositoryImpl {
    /// Create a new AttachmentRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl AttachmentRepository for AttachmentRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<AttachmentModel, TodoError> {
        AttachmentEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
            .and_then(|attachment| {
                attachment
                    .ok_or_else(|| TodoError::NotFound(format!("Attachment {} not found", id)))
            })
    }

    async fn find_all(&self) -> Result<Vec<AttachmentModel>, TodoError> {
        AttachmentEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_item(&self, item_id: &str) -> Result<Vec<AttachmentModel>, TodoError> {
        AttachmentEntity::find()
            .filter(crate::entity::attachments::Column::ItemId.eq(item_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
