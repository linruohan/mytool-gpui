use std::{cmp::max, time::Duration};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    let database_config = &gconfig::get().database();

    let base_url = "sqlite://db.sqlite?mode=rwc".to_owned();

    let mut options = ConnectOptions::new(base_url);
    // let mut options = ConnectOptions::new(format!(
    //     "postgres://{}:{}@{}:{}/{}",
    //     database_config.user(),
    //     database_config.password(),
    //     database_config.host(),
    //     database_config.port(),
    //     database_config.database()
    // ));
    let cpus = num_cpus::get() as u32;
    options
        .min_connections(max(cpus * 4, 10))
        .max_connections(max(cpus * 8, 10))
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(30)) //度数据时间
        .idle_timeout(Duration::from_secs(300)) //连接数不够，空闲时间
        .max_lifetime(Duration::from_secs(3600 * 24))
        .sqlx_logging(false)
        .set_schema_search_path(database_config.schema());
    let db = Database::connect(options).await?;
    db.ping().await?;
    tracing::info!("Database connection successfully");
    // log_database_version(&db).await?;
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
