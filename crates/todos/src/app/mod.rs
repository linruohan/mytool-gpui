use sea_orm::DatabaseConnection;

mod database;
mod database_manager;
mod logger;
mod patch;
mod transaction;

pub use database::init_db;
pub use database_manager::DatabaseManager;
pub use patch::PatchManager;
pub use transaction::TransactionManager;
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
impl AppState {
    pub async fn new(db: DatabaseConnection) -> Self {
        AppState { db }
    }
}
