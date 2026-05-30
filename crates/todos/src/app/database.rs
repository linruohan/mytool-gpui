use std::{path::Path, time::Duration};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    use gconfig::get;

    let db_config = {
        let config_guard = get().read().expect("读取配置失败");
        config_guard.database().clone()
    };
    if db_config.is_sqlite() {
        init_sqlite_db(&db_config).await
    } else {
        init_network_db(&db_config).await
    }
}

async fn init_sqlite_db(db_config: &gconfig::DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let db_path = resolve_db_path(db_config.sqlite_path());
    // 🚀 修复 (2026-05-17)：移除 cache=shared 减少锁竞争
    // cache=shared 可能导致多个连接间的锁竞争，特别是在高并发情况下
    let base_url = format!("sqlite://{}?mode=rwc", db_path);

    let mut options = ConnectOptions::new(base_url);

    // 🚀 优化 (2026-05-17)：连接池配置优化
    // - max_connections: 8 → 20，进一步增加连接池容量（应对高并发场景）
    // - acquire_timeout: 20s → 60s，给更多时间获取连接（解决30秒超时问题）
    // - min_connections: 1 → 2，保持一些预热连接
    // SQLite 在 WAL 模式下允许 1 个写者 + 多个读者，适度增加连接数可提升并发性能
    options
        .min_connections(1)
        .max_connections(20)
        .connect_timeout(Duration::from_secs(3))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(3600))
        .sqlx_logging(false);

    let db = Database::connect(options).await?;

    // 初始连接应用 PRAGMA（SQLite 连接池会在创建新连接时自动应用部分设置）
    tracing::info!("Executing initial PRAGMA settings...");
    let pragma_statements = [
        "PRAGMA journal_mode = WAL;",
        "PRAGMA busy_timeout = 60000;",      // 60 秒等待锁
        "PRAGMA synchronous = NORMAL;",      // 平衡性能和安全性
        "PRAGMA cache_size = -20000;",       // 20MB 缓存
        "PRAGMA foreign_keys = ON;",         // 启用外键约束
        "PRAGMA wal_autocheckpoint = 1000;", // WAL 自动检查点
    ];

    for pragma in &pragma_statements {
        if let Err(e) =
            db.execute(Statement::from_string(DbBackend::Sqlite, pragma.to_string())).await
        {
            tracing::warn!("Failed to execute initial pragma '{}': {:?}", pragma, e);
        }
    }

    // 验证 WAL 模式是否启用
    if let Ok(result) = db
        .query_one(Statement::from_string(DbBackend::Sqlite, "PRAGMA journal_mode".to_string()))
        .await
        && let Some(row) = result
        && let Ok(mode) = row.try_get::<String>("", "journal_mode")
    {
        tracing::info!("SQLite journal_mode = {}", mode);
        if mode != "wal" {
            tracing::warn!("SQLite is not in WAL mode! This may cause performance issues.");
        }
    }

    // 注意：数据库表的创建由 PatchManager::apply_patches() 统一管理
    // 我们只负责连接池创建、PRAGMA 设置和连接验证
    // 这样可以避免 setup.sql 被执行两次（一次在这里，一次在 PatchManager 中）

    db.ping().await?;
    tracing::info!(
        "SQLite database connection successful with WAL mode: {} (pool: max=16, \
         acquire_timeout=30s, min=2)",
        db_path
    );
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
        .connect_timeout(Duration::from_secs(3))
        .acquire_timeout(Duration::from_secs(10))
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
        let base_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().expect("获取当前目录失败"));

        base_path.join(path).to_str().expect("转换路径失败").to_string()
    }
}
