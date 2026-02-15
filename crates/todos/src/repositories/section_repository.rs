//! Section repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{SectionModel, prelude::SectionEntity},
    error::TodoError,
};

/// Repository trait for Section operations
#[async_trait::async_trait]
pub trait SectionRepository {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<SectionModel>, TodoError>;
    async fn find_by_project(&self, project_id: &str) -> Result<Vec<SectionModel>, TodoError>;
}

/// Implementation of SectionRepository
#[derive(Clone, Debug)]
pub struct SectionRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl SectionRepositoryImpl {
    /// Create a new SectionRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl SectionRepository for SectionRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError> {
        SectionEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
            .and_then(|section| {
                section.ok_or_else(|| TodoError::NotFound(format!("Section {} not found", id)))
            })
    }

    async fn find_all(&self) -> Result<Vec<SectionModel>, TodoError> {
        SectionEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_project(&self, project_id: &str) -> Result<Vec<SectionModel>, TodoError> {
        SectionEntity::find()
            .filter(crate::entity::sections::Column::ProjectId.eq(project_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
