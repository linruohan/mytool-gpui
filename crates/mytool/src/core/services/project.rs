use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ProjectModel, error::TodoError};

// ==================== 加载项目 ====================

/// 获取所有的 projects
#[deprecated(since = "2.0", note = "请使用 load_projects_with_store() 方法")]
pub async fn load_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    match Store::new(db).await {
        Ok(store) => store.get_all_projects().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// 使用全局 Store 加载 projects（推荐）
pub async fn load_projects_with_store(store: Arc<Store>) -> Vec<ProjectModel> {
    store.get_all_projects().await.unwrap_or_default()
}

// ==================== 添加项目 ====================

/// 新增 project
#[deprecated(since = "2.0", note = "请使用 add_project_with_store() 方法")]
pub async fn add_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    add_project_with_store(project, store).await
}

/// 使用全局 Store 添加 project（推荐）
pub async fn add_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.insert_project(project.as_ref().clone()).await
}

// ==================== 修改项目 ====================

/// 修改 project
#[deprecated(since = "2.0", note = "请使用 mod_project_with_store() 方法")]
pub async fn mod_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    mod_project_with_store(project, store).await
}

/// 使用全局 Store 修改 project（推荐）
pub async fn mod_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.update_project(project.as_ref().clone()).await
}

// ==================== 删除项目 ====================

/// 删除 project
#[deprecated(since = "2.0", note = "请使用 del_project_with_store() 方法")]
pub async fn del_project(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    let store = Arc::new(Store::new(db).await?);
    del_project_with_store(project, store).await
}

/// 使用全局 Store 删除 project（推荐）
pub async fn del_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_project(&project.id).await
}
