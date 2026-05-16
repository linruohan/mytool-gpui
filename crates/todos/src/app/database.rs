use std::{
    cmp::{max, min},
    path::Path,
    time::Duration,
};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
};

/// 智能分割 SQL 语句
///
/// 解决简单 split(';') 无法处理 CREATE TRIGGER BEGIN...END 块的问题
///
/// 支持的场景：
/// - 普通 SQL 语句以 ; 结尾
/// - CREATE TRIGGER 的 BEGIN...END 块作为整体（内部的 ; 忽略）
/// - 跳过单行注释 (-- ) 和多行注释（/* */）
/// - 处理字符串字面量中的 ;
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_trigger_block = false; // 是否在 BEGIN...END 块中
    let mut begin_depth = 0; // BEGIN 嵌套深度

    while let Some(ch) = chars.next() {
        // 跳过单行注释
        if ch == '-' && chars.peek() == Some(&'-') {
            // 跳过直到行尾
            while let Some(c) = chars.next() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }

        // 跳过多行注释
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next(); // 消耗 *
            loop {
                match chars.next() {
                    Some('*') if chars.peek() == Some(&'/') => {
                        chars.next(); // 消耗 /
                        break;
                    },
                    None => break,
                    _ => {},
                }
            }
            continue;
        }

        // 跳过字符串中的内容（避免误判字符串内的 ;）
        if ch == '\'' || ch == '"' {
            let quote = ch;
            current.push(ch);
            while let Some(c) = chars.next() {
                current.push(c);
                if c == quote {
                    break;
                } // 字符串结束
                if c == '\\' {
                    // 转义字符
                    if let Some(escaped) = chars.next() {
                        current.push(escaped);
                    }
                }
            }
            continue;
        }

        // 追踪 BEGIN...END 块（用于 TRIGGER）
        let upper_ch = ch.to_ascii_uppercase();

        // 检测 BEGIN 关键字（单词边界检查）
        if !in_trigger_block && upper_ch == 'B' {
            let remaining: String = chars.clone().take(4).collect();
            if remaining.to_ascii_uppercase().starts_with("EGIN")
                && matches!(chars.peek(), Some(' ') | Some('\t') | Some('\n') | Some('\r'))
            {
                in_trigger_block = true;
                begin_depth += 1;
            }
        }

        // 检测 END 关键字（后跟 ;）
        if in_trigger_block && upper_ch == 'E' {
            let remaining: String = chars.clone().take(2).collect();
            if remaining.to_ascii_uppercase().starts_with("ND") {
                let after_end: Vec<char> = chars.clone().skip(2).take(5).collect();
                let after_str: String = after_end.iter().collect();
                let trimmed = after_str.trim_start();

                if trimmed.starts_with(';') {
                    // 找到 END; ，结束当前块
                    begin_depth -= 1;
                    if begin_depth == 0 {
                        in_trigger_block = false;
                        current.push(ch);
                        current.extend(remaining.chars());
                        // 添加到 END 后面的空格和分号
                        while let Some(c) = chars.peek() {
                            if *c == ' ' || *c == '\t' || *c == '\r' || *c == '\n' {
                                current.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                        if let Some(';') = chars.peek() {
                            current.push(chars.next().unwrap());
                        }

                        // 完整的 TRIGGER 语句完成
                        let stmt = current.trim().to_string();
                        if !stmt.is_empty() {
                            statements.push(stmt);
                        }
                        current.clear();
                        continue;
                    }
                }
            }
        }

        current.push(ch);

        // 只在非 TRIGGER 块内时，以 ; 作为语句分隔符
        if ch == ';' && !in_trigger_block {
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        }
    }

    // 处理最后一个没有 ; 的语句
    let last_stmt = current.trim().to_string();
    if !last_stmt.is_empty() {
        statements.push(last_stmt);
    }

    statements
}

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
    // 🐛 注意：busy_timeout 只能通过 PRAGMA 设置，不能放 URL（SQLx 不支持此参数）
    let base_url = format!("sqlite://{}?mode=rwc", db_path);

    let mut options = ConnectOptions::new(base_url);

    // 🐛 修复：SQLite 写操作串行化，过多连接只会增加锁竞争
    options
            .min_connections(1)
            .max_connections(5)  // 🐛 修复：从3增加到5，避免连接池耗尽导致 INSERT 卡住
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(5))  // 🐛 修复：从30降到5，快速超时让重试介入
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .sqlx_logging(false);

    let db = Database::connect(options).await?;

    // 🚀 数据库级 PRAGMA（只需设置一次，所有连接共享）
    tracing::info!("Executing initial PRAGMA settings...");
    let pragma_statements = [
        "PRAGMA journal_mode = WAL;",
        "PRAGMA busy_timeout = 60000;", // 兜底确认初始连接也有此设置
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -20000;",
    ];

    for pragma in &pragma_statements {
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

    // 🚀 使用智能 SQL 分割器（正确处理 CREATE TRIGGER BEGIN...END 块）
    let statements = split_sql_statements(setup_sql);
    tracing::info!("Parsed {} SQL statements from setup.sql", statements.len());

    let mut executed_count = 0;
    let mut skipped_count = 0;
    for stmt in &statements {
        // 跳过空语句和纯注释
        if !stmt.is_empty() && !stmt.starts_with("--") && !stmt.starts_with("/*") {
            tracing::info!("Executing SQL: {}...", &stmt[..std::cmp::min(50, stmt.len())]);
            if let Err(e) =
                db.execute(Statement::from_string(DbBackend::Sqlite, stmt.clone())).await
            {
                // 忽略 "table already exists" 和 "trigger already exists" 错误
                let error_str = format!("{:?}", e);
                if error_str.contains("already exists") {
                    tracing::info!("Object already exists, skipping");
                    skipped_count += 1;
                } else if error_str.contains("duplicate column name") {
                    tracing::info!("Column already exists, skipping");
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
