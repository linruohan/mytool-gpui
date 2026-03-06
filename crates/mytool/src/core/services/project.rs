use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ProjectModel, error::TodoError};

// 获取所有的 projects
pub async fn load_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).await.unwrap().get_all_projects().await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 加载 projects
pub async fn load_projects_with_store(store: Arc<Store>) -> Vec<ProjectModel> {
    store.get_all_projects().await.unwrap_or_default()
}

// 新增 project
pub async fn add_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).await?.insert_project(project.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 添加 project
pub async fn add_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.insert_project(project.as_ref().clone()).await
}

// 修改 project
pub async fn mod_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).await?.update_project(project.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 修改 project
pub async fn mod_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.update_project(project.as_ref().clone()).await
}

// 删除 project
pub async fn del_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await?.delete_project(&project.id).await
}

/// 🚀 新增：使用全局 Store 删除 project
pub async fn del_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_project(&project.id).await
}
