use std::rc::Rc;

use sea_orm::DatabaseConnection;
use todos::{
    Store,
    entity::{ItemModel, ProjectModel, SectionModel},
    error::TodoError,
};

pub async fn load_sections(db: DatabaseConnection) -> Vec<SectionModel> {
    Store::new(db).await.sections().await
}
#[allow(unused)]
pub async fn add_section(
    section: Rc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await.insert_section(section.as_ref().clone()).await
}

pub async fn mod_section(
    section: Rc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await.update_section(section.as_ref().clone()).await
}

pub async fn del_section(
    section: Rc<SectionModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await.delete_section(&section.id).await
}

pub async fn get_project_sections(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).await.get_sections_by_project(&project.id).await
}
pub async fn get_sections_by_project_id(
    project_id: &str,
    db: DatabaseConnection,
) -> Vec<SectionModel> {
    Store::new(db).await.get_sections_by_project(project_id).await
}
pub async fn get_section_items(section_id: &str, db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.get_items_by_section(&section_id).await
}
