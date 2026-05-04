//! Item repository for data access operations
//!
//! 实现统一的 BaseRepository 接口，提供标准的 CRUD 操作和特定查询方法。

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

use super::base_repository::{
    BaseRepository, PageableRepository, PagedResult, Pagination, RepositoryBase, RepositoryResult,
    SoftDeletableRepository,
};
use crate::{
    entity::{ItemModel, items, prelude::ItemEntity},
    error::TodoError,
};

/// Item 特有的查询方法 trait
#[async_trait]
pub trait ItemQueryRepository: BaseRepository<ItemModel> {
    /// 根据项目 ID 查找任务
    async fn find_by_project(&self, project_id: &str) -> RepositoryResult<Vec<ItemModel>>;

    /// 根据分区 ID 查找任务
    async fn find_by_section(&self, section_id: &str) -> RepositoryResult<Vec<ItemModel>>;

    /// 根据父任务 ID 查找子任务
    async fn find_by_parent(&self, parent_id: &str) -> RepositoryResult<Vec<ItemModel>>;

    /// 查找已完成的任务
    async fn find_checked(&self) -> RepositoryResult<Vec<ItemModel>>;

    /// 查找未完成的任务
    async fn find_unchecked(&self) -> RepositoryResult<Vec<ItemModel>>;

    /// 查找置顶的任务
    async fn find_pinned(&self) -> RepositoryResult<Vec<ItemModel>>;

    /// 查找今日到期的任务
    async fn find_due_today(&self) -> RepositoryResult<Vec<ItemModel>>;

    /// 查找过期的任务
    async fn find_overdue(&self) -> RepositoryResult<Vec<ItemModel>>;
}

/// Item Repository 实现结构体
#[derive(Clone, Debug)]
pub struct ItemRepositoryImpl {
    base: RepositoryBase,
}

impl ItemRepositoryImpl {
    /// 创建新的 ItemRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { base: RepositoryBase::new(db) }
    }

    /// 将 Model 转换为 ActiveModel 用于插入
    fn to_active_model(item: &ItemModel) -> items::ActiveModel {
        items::ActiveModel {
            id: sea_orm::Set(item.id.clone()),
            content: sea_orm::Set(item.content.clone()),
            description: sea_orm::Set(item.description.clone()),
            due: sea_orm::Set(item.due.clone()),
            added_at: sea_orm::Set(item.added_at),
            completed_at: sea_orm::Set(item.completed_at),
            updated_at: sea_orm::Set(item.updated_at),
            section_id: sea_orm::Set(item.section_id.clone()),
            project_id: sea_orm::Set(item.project_id.clone()),
            parent_id: sea_orm::Set(item.parent_id.clone()),
            priority: sea_orm::Set(item.priority),
            child_order: sea_orm::Set(item.child_order),
            day_order: sea_orm::Set(item.day_order),
            checked: sea_orm::Set(item.checked),
            is_deleted: sea_orm::Set(item.is_deleted),
            collapsed: sea_orm::Set(item.collapsed),
            pinned: sea_orm::Set(item.pinned),
            labels: sea_orm::Set(item.labels.clone()),
            extra_data: sea_orm::Set(item.extra_data.clone()),
            item_type: sea_orm::Set(item.item_type.clone()),
        }
    }
}

#[async_trait]
impl BaseRepository<ItemModel> for ItemRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<ItemModel>> {
        ItemEntity::find_by_id(id)
            .one(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_all(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .order_by_asc(items::Column::DayOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn insert(&self, entity: &ItemModel) -> RepositoryResult<ItemModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .insert(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn update(&self, entity: &ItemModel) -> RepositoryResult<ItemModel> {
        let active_model = Self::to_active_model(entity);
        active_model
            .update(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn delete(&self, id: &str) -> RepositoryResult<bool> {
        let result = ItemEntity::delete_by_id(id)
            .exec(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;
        Ok(result.rows_affected > 0)
    }

    async fn batch_insert(&self, entities: &[ItemModel]) -> RepositoryResult<Vec<ItemModel>> {
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
        ItemEntity::find()
            .count(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl PageableRepository<ItemModel> for ItemRepositoryImpl {
    async fn find_paged(&self, pagination: Pagination) -> RepositoryResult<PagedResult<ItemModel>> {
        let total = self.count().await?;
        let items = ItemEntity::find()
            .order_by_asc(items::Column::DayOrder)
            .offset(pagination.offset())
            .limit(pagination.limit())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(PagedResult::new(items, total, pagination.page, pagination.per_page))
    }
}

#[async_trait]
impl SoftDeletableRepository<ItemModel> for ItemRepositoryImpl {
    async fn soft_delete(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut item) = BaseRepository::find_by_id(self, id).await? {
            item.is_deleted = true;
            self.update(&item).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn restore(&self, id: &str) -> RepositoryResult<bool> {
        if let Some(mut item) = BaseRepository::find_by_id(self, id).await? {
            item.is_deleted = false;
            self.update(&item).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn find_active(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_deleted(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::IsDeleted.eq(true))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }
}

#[async_trait]
impl ItemQueryRepository for ItemRepositoryImpl {
    async fn find_by_project(&self, project_id: &str) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id))
            .filter(items::Column::IsDeleted.eq(false))
            .order_by_asc(items::Column::ChildOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_section(&self, section_id: &str) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::SectionId.eq(section_id))
            .filter(items::Column::IsDeleted.eq(false))
            .order_by_asc(items::Column::ChildOrder)
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_by_parent(&self, parent_id: &str) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::ParentId.eq(parent_id))
            .filter(items::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_checked(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::Checked.eq(true))
            .filter(items::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_unchecked(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::Checked.eq(false))
            .filter(items::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_pinned(&self) -> RepositoryResult<Vec<ItemModel>> {
        ItemEntity::find()
            .filter(items::Column::Pinned.eq(true))
            .filter(items::Column::IsDeleted.eq(false))
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    async fn find_due_today(&self) -> RepositoryResult<Vec<ItemModel>> {
        let today = chrono::Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap();
        let today_end = today.and_hms_opt(23, 59, 59).unwrap();

        Ok(ItemEntity::find()
            .filter(items::Column::Checked.eq(false))
            .filter(items::Column::IsDeleted.eq(false))
            .filter(items::Column::Due.is_not_null())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?
            .into_iter()
            .filter(|item| {
                if let Some(due) = item.due_date() {
                    if let Some(naive) = due.datetime() {
                        return naive >= today_start && naive <= today_end;
                    }
                }
                false
            })
            .collect())
    }

    async fn find_overdue(&self) -> RepositoryResult<Vec<ItemModel>> {
        let now = chrono::Utc::now().naive_utc();

        Ok(ItemEntity::find()
            .filter(items::Column::Checked.eq(false))
            .filter(items::Column::IsDeleted.eq(false))
            .filter(items::Column::Due.is_not_null())
            .all(self.base.database())
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?
            .into_iter()
            .filter(|item| {
                if let Some(due) = item.due_date() {
                    if let Some(naive) = due.datetime() {
                        return naive < now;
                    }
                }
                false
            })
            .collect())
    }
}

/// 旧版兼容接口（已废弃，请使用 BaseRepository）
#[deprecated(since = "2.0", note = "请使用 BaseRepository<ItemModel> trait")]
#[async_trait::async_trait]
pub trait ItemRepository {
    async fn find_by_id(&self, id: &str) -> Result<ItemModel, TodoError>;
    async fn find_all(&self) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_section(&self, section_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_checked(&self) -> Result<Vec<ItemModel>, TodoError>;
    async fn find_unchecked(&self) -> Result<Vec<ItemModel>, TodoError>;
    async fn delete(&self, id: &str) -> Result<(), TodoError>;
}

#[async_trait::async_trait]
impl ItemRepository for ItemRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Result<ItemModel, TodoError> {
        BaseRepository::find_by_id(self, id)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", id)))
    }

    async fn find_all(&self) -> Result<Vec<ItemModel>, TodoError> {
        BaseRepository::find_all(self).await
    }

    async fn find_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemQueryRepository::find_by_project(self, project_id).await
    }

    async fn find_by_section(&self, section_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemQueryRepository::find_by_section(self, section_id).await
    }

    async fn find_by_parent(&self, parent_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        ItemQueryRepository::find_by_parent(self, parent_id).await
    }

    async fn find_checked(&self) -> Result<Vec<ItemModel>, TodoError> {
        ItemQueryRepository::find_checked(self).await
    }

    async fn find_unchecked(&self) -> Result<Vec<ItemModel>, TodoError> {
        ItemQueryRepository::find_unchecked(self).await
    }

    async fn delete(&self, id: &str) -> Result<(), TodoError> {
        BaseRepository::delete(self, id).await?;
        Ok(())
    }
}
