//! Project service for business logic
//!
//! This module provides business logic for Project operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{ProjectActiveModel, ProjectModel, prelude::*, projects, sections},
    error::TodoError,
    repositories::{ProjectRepository, ProjectRepositoryImpl},
    services::{CacheManager, EventBus, ItemService, MetricsCollector, SectionService},
};

/// Service for Project business operations
#[derive(Clone, Debug)]
pub struct ProjectService {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheManager>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    item_service: Arc<ItemService>,
    section_service: Arc<SectionService>,
    project_repo: ProjectRepositoryImpl,
}

impl ProjectService {
    /// Create a new ProjectService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        cache: Arc<CacheManager>,
        metrics: Arc<MetricsCollector>,
        item_service: Arc<ItemService>,
        section_service: Arc<SectionService>,
    ) -> Self {
        let project_repo = ProjectRepositoryImpl::new(db.clone(), cache.clone());
        Self { db, cache, event_bus, metrics, item_service, section_service, project_repo }
    }

    /// Get a project by ID
    pub async fn get_project(&self, id: &str) -> Option<ProjectModel> {
        let result: Result<ProjectModel, TodoError> = self.project_repo.find_by_id(id).await;
        result.ok()
    }

    /// Insert a new project
    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_project");
        let mut active_project: ProjectActiveModel = project.into();
        let project_model = active_project.insert(&*self.db).await?;

        // 更新缓存
        let project_id = project_model.id.clone();
        let project_clone = project_model.clone();
        self.cache.get_or_load_project(&project_id, |_| async move { Ok(project_clone) }).await?;

        self.event_bus.publish(crate::services::event_bus::Event::ProjectCreated(project_id));

        self.metrics.record_operation("insert_project", 1).await;
        Ok(project_model)
    }

    /// Update an existing project
    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let _timer = self.metrics.start_timer("update_project");
        let project_id = project.id.clone();
        let mut active_project: ProjectActiveModel = project.into();
        let result = active_project.update(&*self.db).await?;

        self.cache.invalidate_project(&project_id).await;
        self.event_bus.publish(crate::services::event_bus::Event::ProjectUpdated(project_id));

        self.metrics.record_operation("update_project", 1).await;
        Ok(result)
    }

    /// Delete a project and its children
    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("delete_project");
        let id_clone = id.to_string();

        // 使用迭代方式处理项目，避免递归调用导致的无限大小 future 问题
        let mut projects_to_delete = vec![id.to_string()];

        while let Some(current_id) = projects_to_delete.pop() {
            // 查找当前项目的子项目
            let subprojects = ProjectEntity::find()
                .filter(projects::Column::ParentId.eq(&current_id))
                .all(&*self.db)
                .await?;

            // 将子项目添加到删除队列
            for project in subprojects {
                projects_to_delete.push(project.id);
            }

            // 删除关联的sections
            let sections = SectionEntity::find()
                .filter(sections::Column::ProjectId.eq(&current_id))
                .all(&*self.db)
                .await?;
            for section in sections {
                self.section_service.delete_section(&section.id).await?;
            }

            // 删除关联的items
            if let Ok(items) = self.item_service.get_items_by_project(&current_id).await {
                for item in items {
                    self.item_service.delete_item(&item.id).await?;
                }
            }

            // 删除当前项目
            ProjectEntity::delete_by_id(&current_id).exec(&*self.db).await?;
            self.cache.invalidate_project(&current_id).await;
        }

        self.event_bus.publish(crate::services::event_bus::Event::ProjectDeleted(id_clone));
        self.metrics.record_operation("delete_project", 1).await;
        Ok(())
    }

    /// Archive a project and its items
    pub async fn archive_project(&self, project_id: &str) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("archive_project");
        let project = self
            .get_project(project_id)
            .await
            .ok_or_else(|| TodoError::NotFound("project not found".to_string()))?;

        let archived = !project.is_archived;
        ProjectEntity::update(ProjectActiveModel {
            id: Set(project_id.to_string()),
            is_archived: Set(archived),
            ..project.into()
        })
        .exec(&*self.db)
        .await?;

        // 归档所有items
        let items = self.item_service.get_items_by_project(project_id).await?;
        for item in items {
            self.item_service.archive_item(&item.id, archived).await?;
        }

        self.cache.invalidate_project(project_id).await;
        self.metrics.record_operation("archive_project", 1).await;
        Ok(())
    }

    /// Move project to new parent
    pub async fn move_project(&self, project_id: &str, parent_id: &str) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("move_project");
        let project = self
            .get_project(project_id)
            .await
            .ok_or_else(|| TodoError::NotFound("project not found".to_string()))?;

        ProjectEntity::update(ProjectActiveModel {
            id: Set(project_id.to_string()),
            parent_id: Set(Some(parent_id.to_string())),
            ..project.into()
        })
        .exec(&*self.db)
        .await?;

        self.cache.invalidate_project(project_id).await;
        self.event_bus
            .publish(crate::services::event_bus::Event::ProjectUpdated(project_id.to_string()));

        self.metrics.record_operation("move_project", 1).await;
        Ok(())
    }

    // ==================== Additional Business Logic Methods ====================

    /// Get all projects
    pub async fn get_all_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_projects");
        let projects: Vec<ProjectModel> = ProjectEntity::find().all(&*self.db).await?;
        self.metrics.record_operation("get_all_projects", projects.len()).await;
        Ok(projects)
    }

    /// Get projects by source
    pub async fn get_projects_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<ProjectModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_projects_by_source");
        let projects: Vec<ProjectModel> = ProjectEntity::find()
            .filter(projects::Column::SourceId.eq(source_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_projects_by_source", projects.len()).await;
        Ok(projects)
    }

    /// Get subprojects
    pub async fn get_subprojects(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_subprojects");
        let projects: Vec<ProjectModel> = ProjectEntity::find()
            .filter(projects::Column::ParentId.eq(parent_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_subprojects", projects.len()).await;
        Ok(projects)
    }

    /// Get archived projects
    pub async fn get_archived_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_archived_projects");
        let projects: Vec<ProjectModel> = ProjectEntity::find()
            .filter(projects::Column::IsArchived.eq(true))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_archived_projects", projects.len()).await;
        Ok(projects)
    }

    /// Search projects
    pub async fn search_projects(&self, search_text: &str) -> Result<Vec<ProjectModel>, TodoError> {
        let _timer = self.metrics.start_timer("search_projects");
        let search_lower = search_text.to_lowercase();
        let projects: Vec<ProjectModel> = ProjectEntity::find()
            .filter(projects::Column::Name.contains(&search_lower))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("search_projects", projects.len()).await;
        Ok(projects)
    }

    /// Duplicate a project
    pub async fn duplicate_project(&self, project_id: &str) -> Result<ProjectModel, TodoError> {
        let _timer = self.metrics.start_timer("duplicate_project");
        let project = self
            .get_project(project_id)
            .await
            .ok_or_else(|| TodoError::NotFound("project not found".to_string()))?;

        let mut new_project = project.clone();
        new_project.id = uuid::Uuid::new_v4().to_string();
        new_project.name = format!("{} (copy)", project.name);

        let duplicated_project = self.insert_project(new_project).await?;

        // Duplicate sections
        let sections = self.section_service.get_sections_by_project(project_id).await?;
        for section in sections {
            let mut new_section = section.clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.project_id = Some(duplicated_project.id.clone());
            self.section_service.insert_section(new_section).await?;
        }

        // Duplicate items
        let items = self.item_service.get_items_by_project(project_id).await?;
        for item in items {
            let mut new_item = item.clone();
            new_item.id = uuid::Uuid::new_v4().to_string();
            new_item.project_id = Some(duplicated_project.id.clone());
            self.item_service.insert_item(new_item, true).await?;
        }

        self.metrics.record_operation("duplicate_project", 1).await;
        Ok(duplicated_project)
    }

    /// Get project statistics
    pub async fn get_project_stats(&self, project_id: &str) -> Result<ProjectStats, TodoError> {
        let _timer = self.metrics.start_timer("get_project_stats");

        let items = self.item_service.get_items_by_project(project_id).await?;
        let total_items = items.len();
        let completed_items = items.iter().filter(|i| i.checked).count();
        let pending_items = total_items - completed_items;

        let sections = self.section_service.get_sections_by_project(project_id).await?;
        let total_sections = sections.len();

        let subprojects = self.get_subprojects(project_id).await?;
        let total_subprojects = subprojects.len();

        let stats = ProjectStats {
            project_id: project_id.to_string(),
            total_items,
            completed_items,
            pending_items,
            total_sections,
            total_subprojects,
        };

        self.metrics.record_operation("get_project_stats", 1).await;
        Ok(stats)
    }
}

/// Project statistics
#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub project_id: String,
    pub total_items: usize,
    pub completed_items: usize,
    pub pending_items: usize,
    pub total_sections: usize,
    pub total_subprojects: usize,
}
