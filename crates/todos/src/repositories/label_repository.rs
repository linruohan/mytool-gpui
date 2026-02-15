//! Label repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{LabelModel, prelude::LabelEntity},
    error::TodoError,
};

/// Repository trait for Label operations
#[async_trait::async_trait]
pub trait LabelRepository {
    async fn find_by_id(&self, id: &str) -> Result<LabelModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<LabelModel>, TodoError>;
    async fn find_by_source(&self, source_id: &str) -> Result<Vec<LabelModel>, TodoError>;
    async fn find_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<Option<LabelModel>, TodoError>;
}

/// Implementation of LabelRepository
#[derive(Clone, Debug)]
pub struct LabelRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl LabelRepositoryImpl {
    /// Create a new LabelRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl LabelRepository for LabelRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<LabelModel, TodoError> {
        LabelEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
            .and_then(|label| {
                label.ok_or_else(|| TodoError::NotFound(format!("Label {} not found", id)))
            })
    }

    async fn find_all(&self) -> Result<Vec<LabelModel>, TodoError> {
        LabelEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_source(&self, source_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        LabelEntity::find()
            .filter(crate::entity::labels::Column::SourceId.eq(source_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<Option<LabelModel>, TodoError> {
        LabelEntity::find()
            .filter(crate::entity::labels::Column::Name.eq(name))
            .filter(crate::entity::labels::Column::SourceId.eq(source_id))
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
