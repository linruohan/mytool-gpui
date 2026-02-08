//! Item repository for data access operations

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{ItemModel, prelude::ItemEntity},
    error::TodoError,
    services::CacheManager,
};

/// Repository trait for Item operations
#[async_trait::async_trait]
pub trait ItemRepository {
    async fn find_by_id(&self, id: &str) -> Result<ItemModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_section(&self, section_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_checked(&self) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_unchecked(&self) -> Result<Vec<ItemModel>, TodoError>;
}

/// Implementation of ItemRepository with caching
#[derive(Clone, Debug)]
pub struct ItemRepositoryImpl {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
}

impl ItemRepositoryImpl {
    /// Create a new ItemRepository
    pub fn new(db: Arc<DatabaseConnection>, cache: Arc<CacheManager>) -> Self {
        Self { db, cache }
    }
}

#[async_trait::async_trait]
impl ItemRepository for ItemRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ItemModel, TodoError> {
        let id_clone = id.to_string();
        let db_clone = self.db.clone();
        self.cache
            .get_or_load_item(id, |_| async move {
                ItemEntity::find_by_id(&id_clone)
                    .one(&*db_clone)
                    .await
                    .map_err(|e| TodoError::DatabaseError(e.to_string()))
                    .and_then(|item| {
                        item.ok_or_else(|| {
                            TodoError::NotFound(format!("Item {} not found", id_clone))
                        })
                    })
            })
            .await
    }

    async fn find_all(&self) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find().all(&*self.db).await.map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find()
            .filter(crate::entity::items::Column::ProjectId.eq(project_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_section(&self, section_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find()
            .filter(crate::entity::items::Column::SectionId.eq(section_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find()
            .filter(crate::entity::items::Column::ParentId.eq(parent_id))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_checked(&self) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find()
            .filter(crate::entity::items::Column::Checked.eq(1))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_unchecked(&self) -> Result<Vec<ItemModel>, TodoError> {
        ItemEntity::find()
            .filter(crate::entity::items::Column::Checked.eq(true))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}
