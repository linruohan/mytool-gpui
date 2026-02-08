use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel, SectionModel},
    error::TodoError,
};
// 获取所有的sections
pub async fn load_sections(db: DatabaseConnection) -> Vec<SectionModel> {
    Store::new(db).get_all_sections().await.unwrap_or_default()
}
// 新增section
#[allow(unused)]
pub async fn add_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).insert_section(section.as_ref().clone()).await
}
// 修改section
pub async fn mod_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).update_section(section.as_ref().clone()).await
}
// 删除section
pub async fn del_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).delete_section(&section.id).await
}

// 获取project下的sections
#[allow(unused)]
pub async fn get_project_sections(
    project: Arc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).get_sections_by_project(&project.id).await.unwrap_or_default()
}
// 获取project_id下的sections
#[allow(unused)]
pub async fn get_sections_by_project_id(
    project_id: &str,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).get_sections_by_project(project_id).await.unwrap_or_default()
}
// 获取section下的items
#[allow(unused)]
pub async fn get_section_items(section_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).get_items_by_section(section_id).await.unwrap_or_default()
}
