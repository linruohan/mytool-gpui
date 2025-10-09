use sea_orm::DatabaseConnection;
use todos::Store;
use todos::entity::ProjectModel;

pub async fn get_projects(db: DatabaseConnection) -> Vec<ProjectModel> {
    Store::new(db).await.projects().await
}
