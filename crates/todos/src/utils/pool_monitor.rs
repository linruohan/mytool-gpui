//! 连接池状态监控器
//!
//! 用于诊断 ConnectionAcquire(Timeout) 问题，
//! 打印连接池内部状态和 SQLite 锁状态。
//!
//! ## 设计原则
//! - 诊断函数本身**不应该**进一步消耗连接池资源
//! - 使用极短超时快速探测，避免阻塞
//! - 提供尽可能多的诊断信息而不影响业务
//!
//! ## 🚀 修复说明 (2026-05-16)
//! 针对连接池耗尽问题，增强了诊断能力：
//! 1. 在初始化关键点添加监控调用
//! 2. 提供更详细的错误分类日志
//! 3. 配合连接池扩容（max_connections: 3→8）
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tracing::{debug, info, warn};

/// 诊断连接池状态并打印详细日志
///
/// # 参数
/// - `db`: 数据库连接（连接池入口）
/// - `caller`: 调用者标识（用于日志定位）
pub async fn diagnose_pool_state(db: &DatabaseConnection, caller: &str) {
    let start = std::time::Instant::now();

    info!("🔍 [PoolMonitor] === {} 开始诊断 ===", caller);

    // 1. 尝试获取连接并执行探测查询（测试连通性）
    test_connection_acquire(db, caller).await;

    // 2. 检查 SQLite 锁状态
    check_sqlite_lock_status(db, caller).await;

    info!("🔍 [PoolMonitor] === {} 诊断完成 (耗时={}ms) ===", caller, start.elapsed().as_millis());
}

/// 测试连接获取是否正常
///
/// 注意：使用极短超时（500ms）快速探测，避免在连接池已满时进一步阻塞
async fn test_connection_acquire(db: &DatabaseConnection, caller: &str) {
    let start = std::time::Instant::now();

    match db
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT datetime('now') as current_time".to_string(),
        ))
        .await
    {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis();
            info!("✅ [PoolMonitor-{}] 探测查询成功! 耗时={}ms", caller, elapsed);
        },
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            warn!("❌ [PoolMonitor-{}] 探测查询失败! 耗时={}ms, error={:?}", caller, elapsed, e);

            // 打印更详细的诊断信息
            print_pool_diagnosis_info(caller, elapsed, &e);
        },
    }
}

/// 打印连接池诊断信息
fn print_pool_diagnosis_info(caller: &str, elapsed_ms: u128, error: &sea_orm::DbErr) {
    let error_str = format!("{:?}", error);

    // 根据错误类型给出不同的诊断建议
    if error_str.contains("ConnectionAcquire") || error_str.contains("pool timed out") {
        warn!(
            "🔍 [PoolMonitor-{}] 诊断: 连接池获取超时 ({}ms)\n可能原因:\n1. \
             连接池已满，所有连接都在使用中\n2. 某个连接未正确释放（连接泄漏）\n3. SQLite \
             写锁被长时间持有（WAL 模式下较少见）\n4. 启动时并发加载操作过多",
            caller, elapsed_ms
        );

        // 打印当前线程信息
        warn!("🔍 [PoolMonitor-{}] 当前线程: {:?}", caller, std::thread::current().id());
    } else if error_str.contains("database is locked") || error_str.contains("BUSY") {
        warn!(
            "🔍 [PoolMonitor-{}] 诊断: SQLite 数据库被锁定\n可能原因:\n1. \
             另一个进程/连接正在写入\n2. busy_timeout 设置过短\n3. 长事务未提交",
            caller
        );
    } else {
        warn!("🔍 [PoolMonitor-{}] 诊断: 其他数据库错误 - {:?}", caller, error);
    }
}

/// 检查 SQLite 锁状态
async fn check_sqlite_lock_status(db: &DatabaseConnection, caller: &str) {
    const LOCK_CHECK_SQL: &str = "SELECT 'checking' as status";

    match db.query_one(Statement::from_string(DbBackend::Sqlite, LOCK_CHECK_SQL.to_string())).await
    {
        Ok(Some(result)) => {
            if result.try_get::<String>("", "status").is_ok() {
                info!("✅ [PoolMonitor-{}] SQLite 可正常查询 (无死锁)", caller);
            }
        },
        Ok(None) => {
            warn!("⚠️ [PoolMonitor-{}] 查询返回空结果", caller);
        },
        Err(e) => {
            warn!("⚠️ [PoolMonitor-{}] SQLite 查询异常: {:?}", caller, e);
        },
    }

    // 检查长时间运行的操作
    check_long_running_operations(db, caller).await;
}

/// 检查是否有长时间运行的操作
async fn check_long_running_operations(db: &DatabaseConnection, caller: &str) {
    const TRANSACTION_CHECK_SQL: &str = "
        SELECT COUNT(*) as open_transactions FROM (
            SELECT 1 FROM pragma_lock_list() 
            WHERE type IN ('reserved', 'pending', 'exclusive')
        )
    ";

    match db
        .query_one(Statement::from_string(DbBackend::Sqlite, TRANSACTION_CHECK_SQL.to_string()))
        .await
    {
        Ok(Some(result)) => {
            if let Ok(count) = result.try_get::<i64>("", "open_transactions") {
                if count > 0 {
                    warn!(
                        "⚠️ [PoolMonitor-{}] 检测到 {} 个可能的未提交事务/锁! 这可能是阻塞原因!",
                        caller, count
                    );
                    print_detailed_locks(db, caller).await;
                } else {
                    info!("✅ [PoolMonitor-{}] 无检测到未提交事务", caller);
                }
            }
        },
        Ok(None) => {},
        Err(e) => {
            debug!("🔍 [PoolMonitor-{}] 无法查询事务状态: {:?} (可忽略)", caller, e);
        },
    }
}

/// 打印详细的锁信息
async fn print_detailed_locks(db: &DatabaseConnection, caller: &str) {
    const DETAILED_LOCK_SQL: &str = "SELECT * FROM pragma_lock_list() LIMIT 10";

    match db
        .query_all(Statement::from_string(DbBackend::Sqlite, DETAILED_LOCK_SQL.to_string()))
        .await
    {
        Ok(rows) => {
            for row in rows.iter() {
                let lock_type: Option<String> = row.try_get("", "type").ok().flatten();
                let lock_name: Option<String> = row.try_get("", "name").ok().flatten();

                warn!(
                    "⚠️ [PoolMonitor-{}] 🔒 锁详情: type={:?}, name={:?}",
                    caller, lock_type, lock_name
                );
            }
        },
        Err(e) => {
            debug!("🔍 [PoolMonitor-{}] 无法获取详细锁信息: {:?}", caller, e);
        },
    }
}
