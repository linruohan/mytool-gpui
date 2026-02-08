//! Label repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{LabelModel, prelude::LabelEntity},
    error::TodoError,
    services::CacheManager,
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

/// Implementation of LabelRepository with caching
#[derive(Clone, Debug)]
pub struct LabelRepositoryImpl {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
}

impl LabelRepositoryImpl {
    /// Create a new LabelRepository
    pub fn new(db: Arc<DatabaseConnection>, cache: Arc<CacheManager>) -> Self {
        Self { db, cache }
    }
}

#[async_trait::async_trait]
impl LabelRepository for LabelRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<LabelModel, TodoError> {
        let id_clone = id.to_string();
        let db_clone = self.db.clone();
        self.cache
            .get_or_load_label(id, |_| async move {
                LabelEntity::find_by_id(&id_clone)
                    .one(&*db_clone)
                    .await
                    .map_err(|e| TodoError::DatabaseError(e.to_string()))
                    .and_then(|label| {
                        label.ok_or_else(|| {
                            TodoError::NotFound(format!("Label {} not found", id_clone))
                        })
                    })
            })
            .await
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
