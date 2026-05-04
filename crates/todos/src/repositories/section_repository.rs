//! Section repository for data access operations
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
    entity::{SectionModel, prelude::SectionEntity, sections},
    error::TodoError,
};

/// Section 特有的查询方法 trait
#[async_trait]
pub trait SectionQueryRepository: BaseRepository<SectionModel> {
    /// 根据项目 ID 查找分区
    async fn find_by_project(&self, project_id: &str) -> RepositoryResult<Vec<SectionModel>>;

    /// 查找已归档的分区
    async fn find_archived(&self) -> RepositoryResult<Vec<SectionModel>>;
}

/// Section Repository 实现结构体
#[derive(Clone, Debug)]
pub struct SectionRepositoryImpl {
    base: RepositoryBase,
}

impl SectionRepositoryImpl {
    /// 创建新的 SectionRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { base: RepositoryBase::new(db) }
    }

    /// 将 Model 转换为 ActiveModel 用于插入
    fn to_active_model(section: &SectionModel) -> sections::ActiveModel {
        sections::ActiveModel {
            id: sea_orm::Set(section.id.clone()),
            name: sea_orm::Set(section.name.clone()),
            archived_at: sea_orm::Set(section.archived_at),
            added_at: sea_orm::Set(section.added_at),
            project_id: sea_orm::Set(section.project_id.clone()),
            section_order: sea_orm::Set(section.section_order),
            collapsed: sea_orm::Set(section.collapsed),
            is_deleted: sea_orm::Set(section.is_deleted),
            is_archived: sea_orm::Set(section.is_archived),
            color: sea_orm::Set(section.color.clone()),
            description: sea_orm::Set(section.description.clone()),
            hidded: sea_orm::Set(section.hidded),
        }
    }
}

#[async_trait]
impl BaseRepository<SectionModel> for SectionRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<SectionModel>> {
        SectionEntity::find_by_id(id)
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_all(&self) -> RepositoryResult<Vec<SectionModel>> {
        SectionEntity::find()
            .order_by_asc(sections::Column::SectionOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn insert(&self, entity: &SectionModel) -> RepositoryResult<SectionModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .insert(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn update(&self, entity: &SectionModel) -> RepositoryResult<SectionModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .update(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn delete(&self, id: &str) -> RepositoryResult<bool> {
        let result = SectionEntity::delete_by_id(id)
            .exec(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    async fn batch_insert(&self, entities: &[SectionModel]) -> RepositoryResult<Vec<SectionModel>> {
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
        SectionEntity::find()
            .count(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl PageableRepository<SectionModel> for SectionRepositoryImpl {
    async fn find_paged(
        &self,
        pagination: Pagination,
    ) -> RepositoryResult<PagedResult<SectionModel>> {
        let total = self.count().await?;
        let items = SectionEntity::find()
            .order_by_asc(sections::Column::SectionOrder)
            .offset(pagination.offset())
            .limit(pagination.limit())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(PagedResult::new(items, total, pagination.page, pagination.per_page))
    }
}

#[async_trait]
impl SoftDeletableRepository<SectionModel> for SectionRepositoryImpl {
    async fn soft_delete(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut section) = BaseRepository::find_by_id(self, id).await? {
            section.is_deleted = true;
            self.update(&section).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn restore(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut section) = BaseRepository::find_by_id(self, id).await? {
            section.is_deleted = false;
            self.update(&section).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn find_active(&self) -> RepositoryResult<Vec<SectionModel>> {
        SectionEntity::find()
            .filter(sections::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_deleted(&self) -> RepositoryResult<Vec<SectionModel>> {
        SectionEntity::find()
            .filter(sections::Column::IsDeleted.eq(true))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl SectionQueryRepository for SectionRepositoryImpl {
    async fn find_by_project(&self, project_id: &str) -> RepositoryResult<Vec<SectionModel>> {
        SectionEntity::find()
            .filter(sections::Column::ProjectId.eq(project_id))
            .filter(sections::Column::IsDeleted.eq(false))
            .order_by_asc(sections::Column::SectionOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_archived(&self) -> RepositoryResult<Vec<SectionModel>> {
        SectionEntity::find()
            .filter(sections::Column::IsArchived.eq(true))
            .filter(sections::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

/// 旧版兼容接口（已废弃，请使用 BaseRepository）
#[deprecated(since = "2.0", note = "请使用 BaseRepository<SectionModel> trait")]
#[async_trait::async_trait]
pub trait SectionRepository {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<SectionModel>, TodoError>;
    async fn find_by_project(&self, project_id: &str) -> Result<Vec<SectionModel>, TodoError>;
    async fn delete(&self, id: &str) -> Result<(), TodoError>;
}

#[async_trait::async_trait]
impl SectionRepository for SectionRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<SectionModel, TodoError> {
        BaseRepository::find_by_id(self, id)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Section {} not found", id)))
    }

    async fn find_all(&self) -> Result<Vec<SectionModel>, TodoError> {
        BaseRepository::find_all(self).await
    }

    async fn find_by_project(&self, project_id: &str) -> Result<Vec<SectionModel>, TodoError> {
        SectionQueryRepository::find_by_project(self, project_id).await
    }

    async fn delete(&self, id: &str) -> Result<(), TodoError> {
        BaseRepository::delete(self, id).await?;
        Ok(())
    }
}
