//! Project service for business logic
//!
//! This module provides business logic for Project operations,
//! separating it from data access layer.

use std::{collections::HashSet, sync::Arc};

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{ProjectActiveModel, ProjectModel, items, prelude::*, projects, sections},
    error::TodoError,
    services::ItemService,
};

/// Service for Project business operations
#[derive(Clone, Debug)]
pub struct ProjectService {
    db: Arc<DatabaseConnection>,
    item_service: Arc<ItemService>,
}

impl ProjectService {
    /// Create a new ProjectService
    pub fn new(db: Arc<DatabaseConnection>, item_service: Arc<ItemService>) -> Self {
        Self { db, item_service }
    }

    /// Insert a new project
    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let active_project: ProjectActiveModel = project.into();
        match active_project.insert(&*self.db).await {
            Ok(model) => Ok(model),
            Err(e) => Err(TodoError::DbError(Box::new(e))),
        }
    }

    /// Update an existing project
    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
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

        Ok(result)
    }

    /// Delete a project and its children
    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        // 收集项目树中所有项目 ID
        let mut project_ids = vec![id.to_string()];
        let mut idx = 0;
        while idx < project_ids.len() {
            let current_id = project_ids[idx].clone();
            idx += 1;
            let subprojects = ProjectEntity::find()
                .filter(projects::Column::ParentId.eq(&current_id))
                .all(&*self.db)
                .await?;
            for project in subprojects {
                project_ids.push(project.id);
            }
        }

        // 批量收集需删除的任务 ID（含子任务树）
        let mut item_ids = HashSet::new();

        let project_items = items::Entity::find()
            .filter(items::Column::ProjectId.is_in(project_ids.clone()))
            .all(&*self.db)
            .await?;
        for item in &project_items {
            let ids = self.item_service.collect_descendant_ids(&item.id).await?;
            item_ids.extend(ids);
        }

        let section_ids: Vec<String> = SectionEntity::find()
            .filter(sections::Column::ProjectId.is_in(project_ids.clone()))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|s| s.id)
            .collect();

        if !section_ids.is_empty() {
            let section_items = items::Entity::find()
                .filter(items::Column::SectionId.is_in(section_ids))
                .all(&*self.db)
                .await?;
            for item in &section_items {
                let ids = self.item_service.collect_descendant_ids(&item.id).await?;
                item_ids.extend(ids);
            }
        }

        if !item_ids.is_empty() {
            self.item_service
                .delete_items_by_ids(item_ids.into_iter().collect())
                .await?;
        }

        // Sections 随 Projects FK CASCADE 自动删除
        ProjectEntity::delete_many()
            .filter(projects::Column::Id.is_in(project_ids))
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// Get all projects
    pub async fn get_all_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        let projects: Vec<ProjectModel> = ProjectEntity::find().all(&*self.db).await?;
        Ok(projects)
    }
}
