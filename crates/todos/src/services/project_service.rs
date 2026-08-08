//! Project service for business logic
//!
//! This module provides business logic for Project operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{ProjectActiveModel, ProjectModel, prelude::*, projects, sections},
    error::TodoError,
    services::{EventBus, ItemService, SectionService},
};

/// Service for Project business operations
#[derive(Clone, Debug)]
pub struct ProjectService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    item_service: Arc<ItemService>,
    section_service: Arc<SectionService>,
}

impl ProjectService {
    /// Create a new ProjectService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        item_service: Arc<ItemService>,
        section_service: Arc<SectionService>,
    ) -> Self {
        Self { db, event_bus, item_service, section_service }
    }

    /// Insert a new project
    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let active_project: ProjectActiveModel = project.into();
        match active_project.insert(&*self.db).await {
            Ok(model) => {
                let project_id = model.id.clone();
                self.event_bus
                    .publish(crate::services::event_bus::Event::ProjectCreated(project_id));
                Ok(model)
            },
            Err(e) => Err(TodoError::DbError(Box::new(e))),
        }
    }

    /// Update an existing project
    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let project_id = project.id.clone();

        // 显式设置需要更新的字段
        let active_project = ProjectActiveModel {
            id: Set(project.id),
            name: Set(project.name),
            color: Set(project.color),
            backend_type: Set(project.backend_type),
            inbox_project: Set(project.inbox_project),
            team_inbox: Set(project.team_inbox),
            child_order: Set(project.child_order),
            is_deleted: Set(project.is_deleted),
            is_archived: Set(project.is_archived),
            is_favorite: Set(project.is_favorite),
            shared: Set(project.shared),
            view_style: Set(project.view_style),
            sort_order: Set(project.sort_order),
            parent_id: Set(project.parent_id),
            collapsed: Set(project.collapsed),
            icon_style: Set(project.icon_style),
            emoji: Set(project.emoji),
            show_completed: Set(project.show_completed),
            description: Set(project.description),
            due_date: Set(project.due_date),
            inbox_section_hidded: Set(project.inbox_section_hidded),
            sync_id: Set(project.sync_id),
            source_id: Set(project.source_id),
        };

        let result = active_project.update(&*self.db).await?;

        self.event_bus.publish(crate::services::event_bus::Event::ProjectUpdated(project_id));

        Ok(result)
    }

    /// Delete a project and its children
    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
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
        }

        self.event_bus.publish(crate::services::event_bus::Event::ProjectDeleted(id_clone));
        Ok(())
    }

    /// Get all projects
    pub async fn get_all_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        let projects: Vec<ProjectModel> = ProjectEntity::find().all(&*self.db).await?;
        Ok(projects)
    }
}
