use sea_orm::DatabaseConnection;
use todos::entity::ProjectModel;
use todos::Store;

pub async fn get_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).await.projects().await
}
