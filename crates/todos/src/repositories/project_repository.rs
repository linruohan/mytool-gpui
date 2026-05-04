//! Project repository for data access operations
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
    entity::{ProjectModel, prelude::ProjectEntity, projects},
    error::TodoError,
};

/// Project 特有的查询方法 trait
#[async_trait]
pub trait ProjectQueryRepository: BaseRepository<ProjectModel> {
    /// 根据来源 ID 查找项目
    async fn find_by_source(&self, source_id: &str) -> RepositoryResult<Vec<ProjectModel>>;

    /// 根据父项目 ID 查找子项目
    async fn find_by_parent(&self, parent_id: &str) -> RepositoryResult<Vec<ProjectModel>>;

    /// 查找收藏的项目
    async fn find_favorites(&self) -> RepositoryResult<Vec<ProjectModel>>;

    /// 查找已归档的项目
    async fn find_archived(&self) -> RepositoryResult<Vec<ProjectModel>>;

    /// 查找收件箱项目
    async fn find_inbox_project(&self) -> RepositoryResult<Option<ProjectModel>>;
}

/// Project Repository 实现结构体
#[derive(Clone, Debug)]
pub struct ProjectRepositoryImpl {
    base: RepositoryBase,
}

impl ProjectRepositoryImpl {
    /// 创建新的 ProjectRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { base: RepositoryBase::new(db) }
    }

    /// 将 Model 转换为 ActiveModel 用于插入
    fn to_active_model(project: &ProjectModel) -> projects::ActiveModel {
        projects::ActiveModel {
            id: sea_orm::Set(project.id.clone()),
            name: sea_orm::Set(project.name.clone()),
            color: sea_orm::Set(project.color.clone()),
            backend_type: sea_orm::Set(project.backend_type.clone()),
            inbox_project: sea_orm::Set(project.inbox_project),
            team_inbox: sea_orm::Set(project.team_inbox),
            child_order: sea_orm::Set(project.child_order),
            is_deleted: sea_orm::Set(project.is_deleted),
            is_archived: sea_orm::Set(project.is_archived),
            is_favorite: sea_orm::Set(project.is_favorite),
            shared: sea_orm::Set(project.shared),
            view_style: sea_orm::Set(project.view_style.clone()),
            sort_order: sea_orm::Set(project.sort_order),
            parent_id: sea_orm::Set(project.parent_id.clone()),
            collapsed: sea_orm::Set(project.collapsed),
            icon_style: sea_orm::Set(project.icon_style.clone()),
            emoji: sea_orm::Set(project.emoji.clone()),
            show_completed: sea_orm::Set(project.show_completed),
            description: sea_orm::Set(project.description.clone()),
            due_date: sea_orm::Set(project.due_date.clone()),
            inbox_section_hidded: sea_orm::Set(project.inbox_section_hidded),
            sync_id: sea_orm::Set(project.sync_id.clone()),
            source_id: sea_orm::Set(project.source_id.clone()),
        }
    }
}

#[async_trait]
impl BaseRepository<ProjectModel> for ProjectRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<ProjectModel>> {
        ProjectEntity::find_by_id(id)
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_all(&self) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .order_by_asc(projects::Column::SortOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn insert(&self, entity: &ProjectModel) -> RepositoryResult<ProjectModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .insert(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn update(&self, entity: &ProjectModel) -> RepositoryResult<ProjectModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .update(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn delete(&self, id: &str) -> RepositoryResult<bool> {
        let result = ProjectEntity::delete_by_id(id)
            .exec(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    async fn batch_insert(&self, entities: &[ProjectModel]) -> RepositoryResult<Vec<ProjectModel>> {
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
        ProjectEntity::find()
            .count(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl PageableRepository<ProjectModel> for ProjectRepositoryImpl {
    async fn find_paged(
        &self,
        pagination: Pagination,
    ) -> RepositoryResult<PagedResult<ProjectModel>> {
        let total = self.count().await?;
        let items = ProjectEntity::find()
            .order_by_asc(projects::Column::SortOrder)
            .offset(pagination.offset())
            .limit(pagination.limit())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(PagedResult::new(items, total, pagination.page, pagination.per_page))
    }
}

#[async_trait]
impl SoftDeletableRepository<ProjectModel> for ProjectRepositoryImpl {
    async fn soft_delete(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut project) = BaseRepository::find_by_id(self, id).await? {
            project.is_deleted = true;
            self.update(&project).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn restore(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut project) = BaseRepository::find_by_id(self, id).await? {
            project.is_deleted = false;
            self.update(&project).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn find_active(&self) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_deleted(&self) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::IsDeleted.eq(true))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl ProjectQueryRepository for ProjectRepositoryImpl {
    async fn find_by_source(&self, source_id: &str) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::SourceId.eq(source_id))
            .filter(projects::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_parent(&self, parent_id: &str) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::ParentId.eq(parent_id))
            .filter(projects::Column::IsDeleted.eq(false))
            .order_by_asc(projects::Column::ChildOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_favorites(&self) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::IsFavorite.eq(true))
            .filter(projects::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_archived(&self) -> RepositoryResult<Vec<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::IsArchived.eq(true))
            .filter(projects::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_inbox_project(&self) -> RepositoryResult<Option<ProjectModel>> {
        ProjectEntity::find()
            .filter(projects::Column::InboxProject.eq(1))
            .filter(projects::Column::IsDeleted.eq(false))
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

/// 旧版兼容接口（已废弃，请使用 BaseRepository）
#[deprecated(since = "0.2.0", note = "请使用 BaseRepository<ProjectModel> trait")]
#[async_trait::async_trait]
pub trait ProjectRepository {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_source(&self, source_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError>;
}

#[async_trait::async_trait]
impl ProjectRepository for ProjectRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ProjectModel, TodoError> {
        BaseRepository::find_by_id(self, id)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Project {} not found", id)))
    }

    async fn find_all(&self) -> Result<Vec<ProjectModel>, TodoError> {
        BaseRepository::find_all(self).await
    }

    async fn find_by_source(&self, source_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        ProjectQueryRepository::find_by_source(self, source_id).await
    }

    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        ProjectQueryRepository::find_by_parent(self, parent_id).await
    }
}
