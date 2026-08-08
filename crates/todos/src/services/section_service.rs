//! Section service for business logic
//!
//! This module provides business logic for Section operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{SectionActiveModel, SectionModel},
    error::TodoError,
    repositories::{BaseRepository, SectionRepositoryImpl},
    services::{EventBus, ItemService},
};

/// Service for Section business operations
#[derive(Clone, Debug)]
pub struct SectionService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    section_repo: SectionRepositoryImpl,
    item_service: Arc<ItemService>,
}

impl SectionService {
    /// Create a new SectionService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        item_service: Arc<ItemService>,
    ) -> Self {
        let section_repo = SectionRepositoryImpl::new(db.clone());
        Self { db, event_bus, section_repo, item_service }
    }

    /// Insert a new section
    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let active_section: SectionActiveModel = section.into();
        let section_model = active_section.insert(&*self.db).await?;

        let section_id = section_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::SectionCreated(section_id));

        Ok(section_model)
    }

    /// Update an existing section
    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let section_id = section.id.clone();

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

        self.event_bus.publish(crate::services::event_bus::Event::SectionUpdated(section_id));

        Ok(result)
    }

    /// Delete a section and its items
    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        let section_id_clone = section_id.to_string();

        // 删除关联的items
        let items = self.get_items_by_section(section_id).await?;
        for item in items {
            self.item_service.delete_item(&item.id).await?;
        }

        BaseRepository::delete(&self.section_repo, section_id).await?;
        self.event_bus.publish(crate::services::event_bus::Event::SectionDeleted(section_id_clone));

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
