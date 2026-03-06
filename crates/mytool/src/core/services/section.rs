use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel, SectionModel},
    error::TodoError,
};
// 获取所有的 sections
pub async fn load_sections(db: DatabaseConnection) -> Vec<SectionModel> {
    Store::new(db).await.unwrap().get_all_sections().await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 加载 sections
pub async fn load_sections_with_store(store: Arc<Store>) -> Vec<SectionModel> {
    store.get_all_sections().await.unwrap_or_default()
}

// 新增 section
#[allow(unused)]
pub async fn add_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await?.insert_section(section.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 添加 section
#[allow(unused)]
pub async fn add_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.insert_section(section.as_ref().clone()).await
}

// 修改 section
pub async fn mod_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await?.update_section(section.as_ref().clone()).await
}

/// 🚀 新增：使用全局 Store 修改 section
pub async fn mod_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.update_section(section.as_ref().clone()).await
}

// 删除 section
pub async fn del_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await?.delete_section(&section.id).await
}

/// 🚀 新增：使用全局 Store 删除 section
pub async fn del_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_section(&section.id).await
}

// 获取 project 下的 sections
#[allow(unused)]
pub async fn get_project_sections(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).await.unwrap().get_sections_by_project(&project.id).await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 获取 project 下的 sections
#[allow(unused)]
pub async fn get_project_sections_with_store(
    project: Arc<ProjectModel>,
    store: Arc<Store>,
) -> Vec<SectionModel> {
    store.get_sections_by_project(&project.id).await.unwrap_or_default()
}

// 获取 project_id 下的 sections
#[allow(unused)]
pub async fn get_sections_by_project_id(
    project_id: &str,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).await.unwrap().get_sections_by_project(project_id).await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 获取 sections by project_id
#[allow(unused)]
pub async fn get_sections_by_project_id_with_store(
    project_id: &str,
    store: Arc<Store>,
) -> Vec<SectionModel> {
    store.get_sections_by_project(project_id).await.unwrap_or_default()
}

// 获取 section 下的 items
#[allow(unused)]
pub async fn get_section_items(section_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.unwrap().get_items_by_section(section_id).await.unwrap_or_default()
}

/// 🚀 新增：使用全局 Store 获取 section 下的 items
#[allow(unused)]
pub async fn get_section_items_with_store(section_id: &str, store: Arc<Store>) -> Vec<ItemModel> {
    store.get_items_by_section(section_id).await.unwrap_or_default()
}
