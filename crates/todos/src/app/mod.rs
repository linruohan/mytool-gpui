use sea_orm::DatabaseConnection;

mod database;
mod logger;
pub use database::init_db;
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
impl AppState {
    pub async fn new(db: DatabaseConnection) -> Self {
        AppState { db }
    }
}
