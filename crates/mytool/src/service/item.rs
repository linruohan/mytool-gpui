use sea_orm::DatabaseConnection;
use todos::Store;
use todos::entity::ItemModel;

pub async fn load_items(db: DatabaseConnection) -> Vec<ItemModel> {
    Store::new(db).await.items().await
}
