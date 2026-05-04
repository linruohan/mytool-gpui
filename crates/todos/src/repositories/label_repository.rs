//! Label repository for data access operations
//!
//! 实现统一的 BaseRepository 接口，提供标准的 CRUD 操作和特定查询方法。

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

use super::base_repository::{
    BaseRepository, PageableRepository, PagedResult, Pagination, RepositoryBase, RepositoryResult,
    SoftDeletableRepository,
};
use crate::{
    entity::{LabelModel, labels, prelude::LabelEntity},
    error::TodoError,
};

/// Label 特有的查询方法 trait
#[async_trait]
pub trait LabelQueryRepository: BaseRepository<LabelModel> {
    /// 根据来源 ID 查找标签
    async fn find_by_source(&self, source_id: &str) -> RepositoryResult<Vec<LabelModel>>;

    /// 根据名称和来源 ID 查找标签
    async fn find_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> RepositoryResult<Option<LabelModel>>;

    /// 查找收藏的标签
    async fn find_favorites(&self) -> RepositoryResult<Vec<LabelModel>>;
}

/// Label Repository 实现结构体
#[derive(Clone, Debug)]
pub struct LabelRepositoryImpl {
    base: RepositoryBase,
}

impl LabelRepositoryImpl {
    /// 创建新的 LabelRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { base: RepositoryBase::new(db) }
    }

    /// 将 Model 转换为 ActiveModel 用于插入
    fn to_active_model(label: &LabelModel) -> labels::ActiveModel {
        labels::ActiveModel {
            id: sea_orm::Set(label.id.clone()),
            name: sea_orm::Set(label.name.clone()),
            color: sea_orm::Set(label.color.clone()),
            item_order: sea_orm::Set(label.item_order),
            is_deleted: sea_orm::Set(label.is_deleted),
            is_favorite: sea_orm::Set(label.is_favorite),
            backend_type: sea_orm::Set(label.backend_type.clone()),
            source_id: sea_orm::Set(label.source_id.clone()),
        }
    }
}

#[async_trait]
impl BaseRepository<LabelModel> for LabelRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<LabelModel>> {
        LabelEntity::find_by_id(id)
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_all(&self) -> RepositoryResult<Vec<LabelModel>> {
        LabelEntity::find()
            .order_by_asc(labels::Column::ItemOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn insert(&self, entity: &LabelModel) -> RepositoryResult<LabelModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .insert(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn update(&self, entity: &LabelModel) -> RepositoryResult<LabelModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .update(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn delete(&self, id: &str) -> RepositoryResult<bool> {
        let result = LabelEntity::delete_by_id(id)
            .exec(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    async fn batch_insert(&self, entities: &[LabelModel]) -> RepositoryResult<Vec<LabelModel>> {
        let mut results = Vec::with_capacity(entities.len());
        for entity in entities {
            results.push(self.insert(entity).await?);
        }
        Ok(results)
    }

    async fn batch_delete(&self, ids: &[&str]) -> RepositoryResult<u64> {
        let mut total_deleted = 0;
        for id in ids {
            if BaseRepository::delete(self, id).await? {
                total_deleted += 1;
            }
        }
        Ok(total_deleted)
    }

    async fn count(&self) -> RepositoryResult<u64> {
        LabelEntity::find()
            .count(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl PageableRepository<LabelModel> for LabelRepositoryImpl {
    async fn find_paged(
        &self,
        pagination: Pagination,
    ) -> RepositoryResult<PagedResult<LabelModel>> {
        let total = self.count().await?;
        let items = LabelEntity::find()
            .order_by_asc(labels::Column::ItemOrder)
            .offset(pagination.offset())
            .limit(pagination.limit())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(PagedResult::new(items, total, pagination.page, pagination.per_page))
    }
}

#[async_trait]
impl SoftDeletableRepository<LabelModel> for LabelRepositoryImpl {
    async fn soft_delete(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut label) = BaseRepository::find_by_id(self, id).await? {
            label.is_deleted = true;
            self.update(&label).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn restore(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut label) = BaseRepository::find_by_id(self, id).await? {
            label.is_deleted = false;
            self.update(&label).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn find_active(&self) -> RepositoryResult<Vec<LabelModel>> {
        LabelEntity::find()
            .filter(labels::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_deleted(&self) -> RepositoryResult<Vec<LabelModel>> {
        LabelEntity::find()
            .filter(labels::Column::IsDeleted.eq(true))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl LabelQueryRepository for LabelRepositoryImpl {
    async fn find_by_source(&self, source_id: &str) -> RepositoryResult<Vec<LabelModel>> {
        LabelEntity::find()
            .filter(labels::Column::SourceId.eq(source_id))
            .filter(labels::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> RepositoryResult<Option<LabelModel>> {
        LabelEntity::find()
            .filter(labels::Column::Name.eq(name))
            .filter(labels::Column::SourceId.eq(source_id))
            .filter(labels::Column::IsDeleted.eq(false))
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_favorites(&self) -> RepositoryResult<Vec<LabelModel>> {
        LabelEntity::find()
            .filter(labels::Column::IsFavorite.eq(true))
            .filter(labels::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

/// 旧版兼容接口（已废弃，请使用 BaseRepository）
#[deprecated(since = "2.0", note = "请使用 BaseRepository<LabelModel> trait")]
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
    async fn delete(&self, id: &str) -> Result<u64, TodoError>;
}

#[async_trait::async_trait]
impl LabelRepository for LabelRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<LabelModel, TodoError> {
        BaseRepository::find_by_id(self, id)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Label {} not found", id)))
    }

    async fn find_all(&self) -> Result<Vec<LabelModel>, TodoError> {
        BaseRepository::find_all(self).await
    }

    async fn find_by_source(&self, source_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        LabelQueryRepository::find_by_source(self, source_id).await
    }

    async fn find_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<Option<LabelModel>, TodoError> {
        LabelQueryRepository::find_by_name(self, name, source_id).await
    }

    async fn delete(&self, id: &str) -> Result<u64, TodoError> {
        if BaseRepository::delete(self, id).await? { Ok(1) } else { Ok(0) }
    }
}
