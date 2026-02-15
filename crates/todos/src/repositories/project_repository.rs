//! Project repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ProjectModel, prelude::ProjectEntity},
    error::TodoError,
};

/// Repository trait for Project operations
#[async_trait::async_trait]
pub trait ProjectRepository {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_source(&self, source_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
}

/// Implementation of ProjectRepository
#[derive(Clone, Debug)]
pub struct ProjectRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ProjectRepositoryImpl {
    /// Create a new ProjectRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ProjectRepository for ProjectRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError> {
        ProjectEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
            .and_then(|project| {
                project.ok_or_else(|| TodoError::NotFound(format!("Project {} not found", id)))
            })
    }

    async fn find_all(&self) -> Result<Vec<ProjectModel>, TodoError> {
        ProjectEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_source(&self, source_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        ProjectEntity::find()
            .filter(crate::entity::projects::Column::SourceId.eq(source_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        ProjectEntity::find()
            .filter(crate::entity::projects::Column::ParentId.eq(parent_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
