use sea_orm::DatabaseConnection;

mod database;
mod logger;
mod patch;

pub use database::init_db;
pub use patch::PatchManager;
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
impl AppState {
    pub async fn new(db: DatabaseConnection) -> Self {
        AppState { db }
    }
}
