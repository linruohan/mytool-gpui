use std::sync::Arc;

use todos::{Store, entity::ProjectModel, error::TodoError};

// ==================== 加载项目 ====================

/// 使用全局 Store 加载 projects（推荐）
pub async fn load_projects_with_store(store: Arc<Store>) -> Result<Vec<ProjectModel>, TodoError> {
    store.get_all_projects().await
}

// ==================== 添加项目 ====================

/// 新增 project（推荐）
pub async fn add_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.insert_project(project.as_ref().clone()).await
}

// ==================== 修改项目 ====================

/// 修改 project（推荐）
pub async fn mod_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<ProjectModel, TodoError> {
    store.update_project(project.as_ref().clone()).await
}

// ==================== 删除项目 ====================

/// 删除 project（推荐）
pub async fn del_project_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_project(&project.id).await
}
