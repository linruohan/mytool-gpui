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
    let base_url = format!("sqlite://{}?mode=rwc", db_path);

    let mut options = ConnectOptions::new(base_url);

    // 🚀 关键修复：优化连接池配置
    // SQLite 是单线程的，连接数不宜过多，避免竞争
    options
            .min_connections(1)  // 最小连接数为 1，避免不必要的连接
            .max_connections(10)  // 🔧 加大最大连接数为 10
            .connect_timeout(Duration::from_secs(30)) // 🔧 连接超时时间加大到 30 秒
            .acquire_timeout(Duration::from_secs(60)) // 🔧 获取连接超时时间加大到 60 秒
            .idle_timeout(Duration::from_secs(300)) // 空闲超时时间
            .max_lifetime(Duration::from_secs(1800)) // 最大生命周期
            .sqlx_logging(false); // 🔧 临时关闭 SQL 日志，避免事务提交问题

    let db = Database::connect(options).await?;

    // 🚀 关键修复：在连接建立后执行 PRAGMA 设置以支持并发写入
    // - journal_mode=WAL: 使用 WAL 模式，允许并发读写
    // - busy_timeout: 等待锁释放的超时时间（毫秒）
    // - synchronous=NORMAL: 平衡性能和数据安全
    let pragma_statements = [
        "PRAGMA journal_mode = WAL;",
        "PRAGMA busy_timeout = 60000;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -20000;", // 20MB cache
    ];

    for pragma in pragma_statements {
        db.execute(Statement::from_string(DbBackend::Sqlite, pragma.to_string())).await.map_err(
            |e| {
                tracing::warn!("Failed to execute pragma '{}': {:?}", pragma, e);
                e
            },
        )?;
    }

    // 🚀 关键修复：执行 setup.sql 初始化数据库表
    // 确保所有表（包括 item_labels）都已创建
    tracing::info!("Executing setup.sql to initialize database tables...");
    let setup_sql = include_str!("../../setup.sql");
    let mut executed_count = 0;
    let mut skipped_count = 0;
    for statement in setup_sql.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() && !stmt.starts_with("--") && !stmt.starts_with("/*") {
            tracing::info!("Executing SQL: {}...", &stmt[..std::cmp::min(50, stmt.len())]);
            if let Err(e) =
                db.execute(Statement::from_string(DbBackend::Sqlite, stmt.to_string())).await
            {
                // 忽略 "table already exists" 错误，这是正常的
                let error_str = format!("{:?}", e);
                if error_str.contains("already exists") {
                    tracing::info!("Table already exists, skipping");
                    skipped_count += 1;
                } else {
                    tracing::warn!("Failed to execute setup statement: {:?}", e);
                    skipped_count += 1;
                }
            } else {
                executed_count += 1;
            }
        }
    }
    tracing::info!(
        "Database tables initialized successfully: {} executed, {} skipped",
        executed_count,
        skipped_count
    );

    db.ping().await?;
    tracing::info!("SQLite database connection successful with WAL mode: {}", db_path);
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
        let base_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().expect("获取当前目录失败"));

        base_path.join(path).to_str().expect("转换路径失败").to_string()
    }
}
