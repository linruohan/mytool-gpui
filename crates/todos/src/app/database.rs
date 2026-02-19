use std::{
    cmp::{max, min},
    path::Path,
    time::Duration,
};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    use gconfig::get;

    let config_guard = get().read().expect("读取配置失败");
    let db_config = config_guard.database();

    if db_config.is_sqlite() {
        init_sqlite_db(db_config).await
    } else {
        init_network_db(db_config).await
    }
}

async fn init_sqlite_db(db_config: &gconfig::DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let db_path = resolve_db_path(db_config.sqlite_path());
    let base_url = format!("sqlite://{}?mode=rwc", db_path);

    let mut options = ConnectOptions::new(base_url);
    let pool_size = db_config.pool_size();
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32;

    options
        .min_connections(min(cpus, 5))
        .max_connections(max(pool_size, cpus * 4))
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    let db = Database::connect(options).await?;
    db.ping().await?;
    tracing::info!("SQLite database connection successful: {}", db_path);
    Ok(db)
}

async fn init_network_db(db_config: &gconfig::DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let host = db_config.host().unwrap_or("localhost");
    let port = db_config.port().unwrap_or(5432);
    let user = db_config.user().unwrap_or("postgres");
    let password = db_config.password().unwrap_or("");
    let database = db_config.database();
    let schema = db_config.schema().unwrap_or("public");

    let base_url = format!(
        "{}://{}:{}@{}:{}/{}?schema={}",
        db_config.db_type(),
        user,
        password,
        host,
        port,
        database,
        schema
    );

    let mut options = ConnectOptions::new(base_url);
    let pool_size = db_config.pool_size();

    options
        .min_connections(1)
        .max_connections(pool_size)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(3600))
        .sqlx_logging(true);

    let db = Database::connect(options).await?;
    db.ping().await?;
    tracing::info!("Database connection successful: {}@{}:{}/{}", user, host, port, database);
    Ok(db)
}

fn resolve_db_path(path: &str) -> String {
    let path_obj = Path::new(path);

    if path_obj.is_absolute() {
        path.to_string()
    } else {
        let mut base_path = std::env::current_dir().expect("获取当前目录失败");

        while !base_path.join("crates").exists() {
            if let Some(parent) = base_path.parent() {
                base_path = parent.to_path_buf();
            } else {
                break;
            }
        }

        base_path.join(path).to_str().expect("转换路径失败").to_string()
    }
}
