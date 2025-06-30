use crate::settings;
use sea_orm::DatabaseConnection;

mod database;
mod logger;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
impl AppState {
    pub async fn new(db: DatabaseConnection) -> Self {
        AppState { db }
    }
}

pub async fn init() -> anyhow::Result<()> {
    logger::init();
    tracing::info!("Starting app");
    let db = database::init().await?;
}
