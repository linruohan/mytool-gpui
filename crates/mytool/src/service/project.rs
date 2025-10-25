use sea_orm::DatabaseConnection;
use todos::Store;
use todos::entity::ProjectModel;
use todos::error::TodoError;

pub async fn load_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).await.projects().await
}

pub async fn add_project(
    project: ProjectModel,
    db: DatabaseConnection,
) -> Result<ProjectModel, TodoError> {
    Store::new(db).await.insert_project(project).await
}
