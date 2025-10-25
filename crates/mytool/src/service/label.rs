use sea_orm::DatabaseConnection;
use todos::Store;
use todos::entity::LabelModel;

pub async fn load_labels(db: DatabaseConnection) -> Vec<LabelModel> {
    Store::new(db).await.labels().await
}
