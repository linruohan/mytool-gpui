use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ProjectModel, error::TodoError};

// 获取所有的projects
pub async fn load_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).get_all_projects().await.unwrap_or_default()
}

// 新增project
pub async fn add_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).insert_project(project.as_ref().clone()).await
}
// 修改project
pub async fn mod_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).update_project(project.as_ref().clone()).await
}

// 删除project
pub async fn del_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).delete_project(&project.id).await
}

/// 根据ID删除项目（用于增量更新）
pub async fn del_project_by_id(project_id: &str, db: DatabaseConnection) -> Result<(), TodoError> {
    Store::new(db).delete_project(project_id).await
}
