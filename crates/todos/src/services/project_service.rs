//! Project service for business logic
//!
//! This module provides business logic for Project operations,
//! separating it from data access layer.
#![allow(deprecated)] // 允许使用废弃的 Repository trait（兼容层，待迁移到 BaseRepository）

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{
    entity::{ProjectActiveModel, ProjectModel, prelude::*, projects, sections},
    error::TodoError,
    repositories::{ProjectRepository, ProjectRepositoryImpl},
    services::{EventBus, ItemService, MetricsCollector, SectionService},
};

/// Service for Project business operations
#[derive(Clone, Debug)]
pub struct ProjectService {
    db: Arc<DatabaseConnection>,
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
        metrics: Arc<MetricsCollector>,
        item_service: Arc<ItemService>,
        section_service: Arc<SectionService>,
    ) -> Self {
        let project_repo = ProjectRepositoryImpl::new(db.clone());
        Self { db, event_bus, metrics, item_service, section_service, project_repo }
    }

    /// Get a project by ID
    pub async fn get_project(&self, id: &str) -> Option<ProjectModel> {
        let result: Result<ProjectModel, TodoError> = self.project_repo.find_by_id(id).await;
        result.ok()
    }

    /// Insert a new project
    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_project");
        let active_project: ProjectActiveModel = project.into();
        match active_project.insert(&*self.db).await {
            Ok(model) => {
                let project_id = model.id.clone();
                self.event_bus
                    .publish(crate::services::event_bus::Event::ProjectCreated(project_id));
                self.metrics.record_operation("insert_project", 1).await;
                Ok(model)
            },
            Err(e) => Err(TodoError::DbError(e)),
        }
    }

    /// 🐛 使用独立 SQLite 连接插入项目（绕过 Sea-ORM 连接池）
    ///
    /// 当连接池被其他操作占满时（如 Item 批量保存），此方法可以
    /// 创建一个完全独立的 SQLite 连接来执行 INSERT，避免 ConnectionAcquire(Timeout)。
    ///
    /// 注意：此方法不经过 before_save 钩子，需要手动生成 ID。
    pub fn insert_project_direct(
        db_path: &str,
        project: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        let new_id = Uuid::new_v4().to_string();

        // 创建专用的 Runtime，彻底避免与主 DB Runtime 竞争工作线程
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| TodoError::DatabaseError(format!("创建专用Runtime失败: {}", e)))?;

        rt.block_on(async move {
            // 用 Sea-ORM 创建独立的 DatabaseConnection（独立的连接池，不受主池影响）
            let db_url = format!("sqlite://{}?mode=rwc", db_path);
            let mut opts = sea_orm::ConnectOptions::new(db_url);
            opts.max_connections(1)
                .connect_timeout(std::time::Duration::from_secs(10))
                .acquire_timeout(std::time::Duration::from_secs(10));
            let db = sea_orm::Database::connect(opts)
                .await
                .map_err(|e| TodoError::DatabaseError(format!("独立DB连接失败: {}", e)))?;

            // 设置 PRAGMA（通过 ConnectionTrait 访问 execute 方法）
            use sea_orm::{ConnectionTrait, DbBackend, Statement};
            for pragma in &[
                "PRAGMA journal_mode = WAL",
                "PRAGMA busy_timeout = 30000",
                "PRAGMA synchronous = NORMAL",
            ] {
                db.execute(Statement::from_string(DbBackend::Sqlite, pragma.to_string()))
                    .await
                    .map_err(|e| TodoError::DatabaseError(format!("PRAGMA失败: {}", e)))?;
            }

            // 用 Sea-ORM ActiveModel 插入（享受完整的 ORM 功能）
            let mut active: ProjectActiveModel = project.into();
            active.id = Set(new_id.clone());
            active
                .insert(&db)
                .await
                .map_err(|e| TodoError::DatabaseError(format!("INSERT失败: {}", e)))
        })
    }

    /// Update an existing project
    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let _timer = self.metrics.start_timer("update_project");
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
            .ok_or_else(|| TodoError::not_found("Project").with_entity("Project", project_id))?;

        let archived = !project.is_archived;

        // 显式设置所有字段，避免使用 ..project.into()
        ProjectEntity::update(ProjectActiveModel {
            id: Set(project_id.to_string()),
            name: Set(project.name),
            color: Set(project.color),
            backend_type: Set(project.backend_type),
            inbox_project: Set(project.inbox_project),
            team_inbox: Set(project.team_inbox),
            child_order: Set(project.child_order),
            is_deleted: Set(project.is_deleted),
            is_archived: Set(archived),
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
        })
        .exec(&*self.db)
        .await?;

        let items_count = self.item_service.get_items_by_project(project_id).await?.len();
        tracing::info!(
            "Project {} archived with {} items, but Items table has no is_archived field, so \
             items are not actually archived.",
            project_id,
            items_count
        );

        self.metrics.record_operation("archive_project", 1).await;
        Ok(())
    }

    /// Move project to new parent
    pub async fn move_project(&self, project_id: &str, parent_id: &str) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("move_project");
        let project = self
            .get_project(project_id)
            .await
            .ok_or_else(|| TodoError::not_found("Project").with_entity("Project", project_id))?;

        // 显式设置所有字段，避免使用 ..project.into()
        ProjectEntity::update(ProjectActiveModel {
            id: Set(project_id.to_string()),
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
            parent_id: Set(Some(parent_id.to_string())),
            collapsed: Set(project.collapsed),
            icon_style: Set(project.icon_style),
            emoji: Set(project.emoji),
            show_completed: Set(project.show_completed),
            description: Set(project.description),
            due_date: Set(project.due_date),
            inbox_section_hidded: Set(project.inbox_section_hidded),
            sync_id: Set(project.sync_id),
            source_id: Set(project.source_id),
        })
        .exec(&*self.db)
        .await?;

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
            .ok_or_else(|| TodoError::not_found("Project").with_entity("Project", project_id))?;

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
