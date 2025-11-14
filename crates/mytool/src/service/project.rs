use std::rc::Rc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::ProjectModel, error::TodoError};

pub async fn load_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).await.projects().await
}

pub async fn add_project(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).await.insert_project(project.as_ref().clone()).await
}

pub async fn mod_project(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).await.update_project(project.as_ref().clone()).await
}

pub async fn del_project(
    project: Rc<ProjectModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await.delete_project(&project.id).await
}
