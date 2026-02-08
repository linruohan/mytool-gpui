//! Project repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ProjectModel, prelude::ProjectEntity},
    error::TodoError,
    services::CacheManager,
};

/// Repository trait for Project operations
#[async_trait::async_trait]
pub trait ProjectRepository {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_source(&self, source_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
}

/// Implementation of ProjectRepository with caching
#[derive(Clone, Debug)]
pub struct ProjectRepositoryImpl {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
}

impl ProjectRepositoryImpl {
    /// Create a new ProjectRepository
    pub fn new(db: Arc<DatabaseConnection>, cache: Arc<CacheManager>) -> Self {
        Self { db, cache }
    }
}

#[async_trait::async_trait]
impl ProjectRepository for ProjectRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError> {
        let id_clone = id.to_string();
        let db_clone = self.db.clone();
        self.cache
            .get_or_load_project(id, |_| async move {
                ProjectEntity::find_by_id(&id_clone)
                    .one(&*db_clone)
                    .await
                    .map_err(|e| TodoError::DatabaseError(e.to_string()))
                    .and_then(|project| {
                        project.ok_or_else(|| {
                            TodoError::NotFound(format!("Project {} not found", id_clone))
                        })
                    })
            })
            .await
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
