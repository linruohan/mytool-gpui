//! Section service for business logic
//!
//! This module provides business logic for Section operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{SectionActiveModel, SectionModel, prelude::*, sections},
    error::TodoError,
    repositories::{SectionRepository, SectionRepositoryImpl},
    services::{CacheManager, EventBus, MetricsCollector},
};

/// Service for Section business operations
#[derive(Clone, Debug)]
pub struct SectionService {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    section_repo: SectionRepositoryImpl,
}

impl SectionService {
    /// Create a new SectionService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        cache: Arc<CacheManager>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let section_repo = SectionRepositoryImpl::new(db.clone(), cache.clone());
        Self { db, cache, event_bus, metrics, section_repo }
    }

    /// Get a section by ID
    pub async fn get_section(&self, id: &str) -> Option<SectionModel> {
        let result: Result<SectionModel, TodoError> = self.section_repo.find_by_id(id).await;
        result.ok()
    }

    /// Insert a new section
    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_section");
        let mut active_section: SectionActiveModel = section.into();
        let section_model = active_section.insert(&*self.db).await?;

        // 更新缓存
        let section_id = section_model.id.clone();
        let section_clone = section_model.clone();
        self.cache.get_or_load_section(&section_id, |_| async move { Ok(section_clone) }).await?;

        self.event_bus.publish(crate::services::event_bus::Event::SectionCreated(section_id));

        self.metrics.record_operation("insert_section", 1).await;
        Ok(section_model)
    }

    /// Update an existing section
    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let _timer = self.metrics.start_timer("update_section");
        let section_id = section.id.clone();
        let mut active_section: SectionActiveModel = section.into();
        let result = active_section.update(&*self.db).await?;

        self.cache.invalidate_section(&section_id).await;
        self.event_bus.publish(crate::services::event_bus::Event::SectionUpdated(section_id));

        (*self.metrics).record_operation("update_section", 1).await;
        Ok(result)
    }

    /// Delete a section and its items
    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        let _timer = (*self.metrics).start_timer("delete_section");
        let section_id_clone = section_id.to_string();

        // 删除关联的items
        let items = self.get_items_by_section(section_id).await?;
        for item in items {
            // TODO: 使用ItemService删除
        }

        SectionEntity::delete_by_id(section_id).exec(&*self.db).await?;
        self.cache.invalidate_section(section_id).await;
        self.event_bus.publish(crate::services::event_bus::Event::SectionDeleted(section_id_clone));

        (*self.metrics).record_operation("delete_section", 1).await;
        Ok(())
    }

    /// Move section to another project
    pub async fn move_section(&self, section_id: &str, project_id: &str) -> Result<(), TodoError> {
        let _timer = (*self.metrics).start_timer("move_section");
        let section = self
            .get_section(section_id)
            .await
            .ok_or_else(|| TodoError::NotFound("section not found".to_string()))?;

        SectionEntity::update(SectionActiveModel {
            id: Set(section_id.to_string()),
            project_id: Set(Some(project_id.to_string())),
            ..section.into()
        })
        .exec(&*self.db)
        .await?;

        self.cache.invalidate_section(section_id).await;
        self.event_bus
            .publish(crate::services::event_bus::Event::SectionUpdated(section_id.to_string()));

        (*self.metrics).record_operation("move_section", 1).await;
        Ok(())
    }

    /// Archive a section and its items
    pub async fn archive_section(&self, section_id: &str, archived: bool) -> Result<(), TodoError> {
        let _timer = (*self.metrics).start_timer("archive_section");
        let section = self
            .get_section(section_id)
            .await
            .ok_or_else(|| TodoError::NotFound("section not found".to_string()))?;

        let archived_new = if section.is_archived == archived { !archived } else { archived };
        let active_model = SectionActiveModel {
            is_archived: Set(archived_new),
            archived_at: Set(Some(chrono::Utc::now().naive_utc())),
            ..section.into()
        };
        active_model.update(&*self.db).await?;

        // 归档所有items
        let items = self.get_items_by_section(section_id).await?;
        for item in items {
            // TODO: 使用ItemService归档
        }

        self.cache.invalidate_section(section_id).await;
        (*self.metrics).record_operation("archive_section", 1).await;
        Ok(())
    }

    // ==================== Additional Business Logic Methods ====================

    /// Get all sections
    pub async fn get_all_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        let _timer = (*self.metrics).start_timer("get_all_sections");
        let sections = SectionEntity::find().all(&*self.db).await?;
        (*self.metrics).record_operation("get_all_sections", sections.len()).await;
        Ok(sections)
    }

    /// Get sections by project
    pub async fn get_sections_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<SectionModel>, TodoError> {
        let _timer = (*self.metrics).start_timer("get_sections_by_project");
        let sections = SectionEntity::find()
            .filter(sections::Column::ProjectId.eq(project_id))
            .all(&*self.db)
            .await?;
        (*self.metrics).record_operation("get_sections_by_project", sections.len()).await;
        Ok(sections)
    }

    /// Get items in a section
    pub async fn get_items_by_section(
        &self,
        section_id: &str,
    ) -> Result<Vec<crate::entity::ItemModel>, TodoError> {
        let _timer = (*self.metrics).start_timer("get_items_by_section");
        use crate::entity::items;
        let items: Vec<crate::entity::ItemModel> = items::Entity::find()
            .filter(items::Column::SectionId.eq(section_id))
            .all(&*self.db)
            .await?;
        (*self.metrics).record_operation("get_items_by_section", items.len()).await;
        Ok(items)
    }

    /// Get archived sections
    pub async fn get_archived_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        let _timer = (*self.metrics).start_timer("get_archived_sections");
        let sections = SectionEntity::find()
            .filter(sections::Column::IsArchived.eq(true))
            .all(&*self.db)
            .await?;
        (*self.metrics).record_operation("get_archived_sections", sections.len()).await;
        Ok(sections)
    }

    /// Search sections
    pub async fn search_sections(&self, search_text: &str) -> Result<Vec<SectionModel>, TodoError> {
        let _timer = (*self.metrics).start_timer("search_sections");
        let search_lower = search_text.to_lowercase();
        let sections = SectionEntity::find()
            .filter(sections::Column::Name.contains(&search_lower))
            .all(&*self.db)
            .await?;
        (*self.metrics).record_operation("search_sections", sections.len()).await;
        Ok(sections)
    }

    /// Duplicate a section
    pub async fn duplicate_section(&self, section_id: &str) -> Result<SectionModel, TodoError> {
        let _timer = (*self.metrics).start_timer("duplicate_section");
        let section = self
            .get_section(section_id)
            .await
            .ok_or_else(|| TodoError::NotFound("section not found".to_string()))?;

        let mut new_section = section.clone();
        new_section.id = uuid::Uuid::new_v4().to_string();
        new_section.name = format!("{} (copy)", section.name);
        new_section.added_at = chrono::Utc::now().naive_utc();

        let duplicated_section = self.insert_section(new_section).await?;

        // Duplicate items
        let items = self.get_items_by_section(section_id).await?;
        for item in items {
            let mut new_item = item.clone();
            new_item.id = uuid::Uuid::new_v4().to_string();
            new_item.section_id = Some(duplicated_section.id.clone());
            // TODO: 使用ItemService插入
        }

        (*self.metrics).record_operation("duplicate_section", 1).await;
        Ok(duplicated_section)
    }

    /// Get section statistics
    pub async fn get_section_stats(&self, section_id: &str) -> Result<SectionStats, TodoError> {
        let _timer = (*self.metrics).start_timer("get_section_stats");

        let items = self.get_items_by_section(section_id).await?;
        let total_items = items.len();
        let completed_items = items.iter().filter(|i| i.checked).count();
        let pending_items = total_items - completed_items;

        let stats = SectionStats {
            section_id: section_id.to_string(),
            total_items,
            completed_items,
            pending_items,
        };

        (*self.metrics).record_operation("get_section_stats", 1).await;
        Ok(stats)
    }
}

/// Section statistics
#[derive(Debug, Clone)]
pub struct SectionStats {
    pub section_id: String,
    pub total_items: usize,
    pub completed_items: usize,
    pub pending_items: usize,
}
