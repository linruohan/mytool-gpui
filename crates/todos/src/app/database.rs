use std::{
    cmp::{max, min},
    time::Duration,
};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    let config_guard = gconfig::get().read().expect("读取配置失败");
    let database_config = config_guard.database();

    let base_url = "sqlite://db.sqlite?mode=rwc".to_owned();

    let mut options = ConnectOptions::new(base_url);
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32;
    options
        .min_connections(min(cpus, 5))
        .max_connections(cpus * 4)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    if let Some(schema) = database_config.schema() {
        options.set_schema_search_path(schema);
    }
    let db = Database::connect(options).await?;
    db.ping().await?;
    tracing::info!("Database connection successfully");
    Ok(db)
}
#[allow(dead_code)]
async fn log_database_version(db: &DatabaseConnection) -> anyhow::Result<()> {
    let version_result = db
        .query_one(Statement::from_string(DbBackend::Sqlite, String::from("SELECT version()")))
        .await?
        .ok_or_else(|| anyhow::anyhow!("failed to get Database version "))?;
    tracing::info!("Database version: {}", version_result.try_get_by_index::<String>(0)?);
    Ok(())
}
