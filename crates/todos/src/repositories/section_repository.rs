//! Section repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{SectionModel, prelude::SectionEntity},
    error::TodoError,
    services::CacheManager,
};

/// Repository trait for Section operations
#[async_trait::async_trait]
pub trait SectionRepository {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<SectionModel>, TodoError>;
    async fn find_by_project(&self, project_id: &str) -> Result<Vec<SectionModel>, TodoError>;
}

/// Implementation of SectionRepository with caching
#[derive(Clone, Debug)]
pub struct SectionRepositoryImpl {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
}

impl SectionRepositoryImpl {
    /// Create a new SectionRepository
    pub fn new(db: Arc<DatabaseConnection>, cache: Arc<CacheManager>) -> Self {
        Self { db, cache }
    }
}

#[async_trait::async_trait]
impl SectionRepository for SectionRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError> {
        let id_clone = id.to_string();
        let db_clone = self.db.clone();
        self.cache
            .get_or_load_section(id, |_| async move {
                SectionEntity::find_by_id(&id_clone)
                    .one(&*db_clone)
                    .await
                    .map_err(|e| TodoError::DatabaseError(e.to_string()))
                    .and_then(|section| {
                        section.ok_or_else(|| {
                            TodoError::NotFound(format!("Section {} not found", id_clone))
                        })
                    })
            })
            .await
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
