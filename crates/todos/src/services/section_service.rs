//! Section service for business logic
//!
//! This module provides business logic for Section operations,
//! separating it from data access layer.

use std::{collections::HashSet, sync::Arc};

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{SectionActiveModel, SectionModel},
    error::TodoError,
    repositories::{BaseRepository, SectionRepositoryImpl},
    services::ItemService,
};

/// Service for Section business operations
#[derive(Clone, Debug)]
pub struct SectionService {
    db: Arc<DatabaseConnection>,
    section_repo: SectionRepositoryImpl,
    item_service: Arc<ItemService>,
}

impl SectionService {
    /// Create a new SectionService
    pub fn new(db: Arc<DatabaseConnection>, item_service: Arc<ItemService>) -> Self {
        let section_repo = SectionRepositoryImpl::new(db.clone());
        Self { db, section_repo, item_service }
    }

    /// Insert a new section
    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let active_section: SectionActiveModel = section.into();
        let section_model = active_section.insert(&*self.db).await?;

        Ok(section_model)
    }

    /// Update an existing section
    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        // 显式设置需要更新的字段
        let active_section = SectionActiveModel {
            id: Set(section.id),
            name: Set(section.name),
            archived_at: Set(section.archived_at),
            added_at: Set(section.added_at),
            project_id: Set(section.project_id),
            section_order: Set(section.section_order),
            collapsed: Set(section.collapsed),
            is_deleted: Set(section.is_deleted),
            is_archived: Set(section.is_archived),
            color: Set(section.color),
            description: Set(section.description),
            hidded: Set(section.hidded),
        };

        let result = active_section.update(&*self.db).await?;

        Ok(result)
    }

    /// Delete a section and its items
    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        let section_items = self.get_items_by_section(section_id).await?;
        let mut item_ids = HashSet::new();
        for item in section_items {
            let ids = self.item_service.collect_descendant_ids(&item.id).await?;
            item_ids.extend(ids);
        }

        if !item_ids.is_empty() {
            self.item_service
                .delete_items_by_ids(item_ids.into_iter().collect())
                .await?;
        }

        BaseRepository::delete(&self.section_repo, section_id).await?;

        Ok(())
    }

    /// Get all sections
    pub async fn get_all_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        let sections = BaseRepository::find_all(&self.section_repo).await?;
        Ok(sections)
    }

    /// Get items in a section
    pub async fn get_items_by_section(
        &self,
        section_id: &str,
    ) -> Result<Vec<crate::entity::ItemModel>, TodoError> {
        use crate::entity::items;
        let items: Vec<crate::entity::ItemModel> = items::Entity::find()
            .filter(items::Column::SectionId.eq(section_id))
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}
