use std::{collections::HashMap, sync::Arc, time::Duration};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};
use tokio::sync::RwLock;

use crate::error::TodoError;

#[derive(Debug)]
pub struct DatabaseManager {
    connections: Arc<RwLock<HashMap<String, DatabaseConnection>>>,
    config: gconfig::DatabaseConfig,
}

impl DatabaseManager {
    pub async fn new(config: gconfig::DatabaseConfig) -> Result<Self, DbErr> {
        let connections = Arc::new(RwLock::new(HashMap::new()));
        Ok(Self { connections, config })
    }

    pub async fn get_connection(&self, db_type: &str) -> Result<DatabaseConnection, DbErr> {
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get(db_type) {
            return Ok(conn.clone());
        }

        let conn = if db_type == "sqlite" {
            self.init_sqlite_db().await?
        } else {
            self.init_network_db().await?
        };

        connections.insert(db_type.to_string(), conn.clone());
        Ok(conn)
    }

    pub async fn health_check(&self) -> Result<bool, DbErr> {
        let connections = self.connections.read().await;

        for (_, conn) in connections.iter() {
            if let Err(e) = conn.ping().await {
                tracing::warn!("Database health check failed: {:?}", e);
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn init_sqlite_db(&self) -> Result<DatabaseConnection, DbErr> {
        let db_path = self.resolve_db_path(self.config.sqlite_path());
        let base_url = format!("sqlite://{}?mode=rwc", db_path);

        let mut options = ConnectOptions::new(base_url);

        options
            .min_connections(1)
            .max_connections(10)
            .connect_timeout(Duration::from_secs(30))
            .acquire_timeout(Duration::from_secs(60))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .sqlx_logging(false);

        let db = Database::connect(options).await?;

        let pragma_statements = [
            "PRAGMA journal_mode = WAL;",
            "PRAGMA busy_timeout = 60000;",
            "PRAGMA synchronous = NORMAL;",
            "PRAGMA cache_size = -20000;",
        ];

        for pragma in pragma_statements {
            db.execute(Statement::from_string(DbBackend::Sqlite, pragma.to_string()))
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to execute pragma '{}': {:?}", pragma, e);
                    e
                })?;
        }

        db.ping().await?;
        tracing::info!("SQLite database connection successful with WAL mode: {}", db_path);
        Ok(db)
    }

    async fn init_network_db(&self) -> Result<DatabaseConnection, DbErr> {
        let host = self.config.host().unwrap_or("localhost");
        let port = self.config.port().unwrap_or(5432);
        let user = self.config.user().unwrap_or("postgres");
        let password = self.config.password().unwrap_or("");
        let database = self.config.database();
        let schema = self.config.schema().unwrap_or("public");

        let base_url = format!(
            "{}://{}:{}@{}:{}/{}?schema={}",
            self.config.db_type(),
            user,
            password,
            host,
            port,
            database,
            schema
        );

        let mut options = ConnectOptions::new(base_url);
        let pool_size = self.config.pool_size();

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

    fn resolve_db_path(&self, path: &str) -> String {
        use std::path::Path;

        let path_obj = Path::new(path);

        if path_obj.is_absolute() {
            path.to_string()
        } else {
            let base_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().expect("获取当前目录失败"));

            base_path.join(path).to_str().expect("转换路径失败").to_string()
        }
    }
}
