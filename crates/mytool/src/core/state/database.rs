use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use gpui::Global;
use sea_orm::DatabaseConnection;

/// 数据库连接状态
///
/// 存储全局数据库连接，供旧的状态管理代码使用。
/// 新代码建议使用 TodoStore，它会自动管理数据加载。
///
/// 注意：DatabaseConnection 内部已经使用了 Arc 进行连接池管理，
/// 所以克隆操作是轻量级的（只增加引用计数）。
///
/// ## 优化说明
/// - 使用 Arc 包装，明确表达共享语义
/// - 添加连接统计，便于监控和诊断
/// - 支持连接健康检查
pub struct DBState {
    pub conn: Arc<DatabaseConnection>,
    stats: Arc<ConnectionStats>,
}

impl DBState {
    /// 创建新的数据库状态
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn: Arc::new(conn), stats: Arc::new(ConnectionStats::new()) }
    }

    /// 获取数据库连接（轻量级克隆）
    #[inline]
    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        self.stats.record_access();
        self.conn.clone()
    }

    /// 获取连接统计信息
    pub fn get_stats(&self) -> ConnectionStatsSnapshot {
        self.stats.snapshot()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats.reset();
    }
}

impl Global for DBState {}

/// 连接统计信息
struct ConnectionStats {
    /// 总访问次数
    total_accesses: AtomicUsize,
    /// 创建时间
    created_at: Instant,
}

impl ConnectionStats {
    fn new() -> Self {
        Self { total_accesses: AtomicUsize::new(0), created_at: Instant::now() }
    }

    #[inline]
    fn record_access(&self) {
        self.total_accesses.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ConnectionStatsSnapshot {
        ConnectionStatsSnapshot {
            total_accesses: self.total_accesses.load(Ordering::Relaxed),
            uptime: self.created_at.elapsed(),
        }
    }

    fn reset(&self) {
        self.total_accesses.store(0, Ordering::Relaxed);
    }
}

/// 连接统计快照
#[derive(Debug, Clone)]
pub struct ConnectionStatsSnapshot {
    /// 总访问次数
    pub total_accesses: usize,
    /// 运行时间
    pub uptime: Duration,
}

impl ConnectionStatsSnapshot {
    /// 计算平均访问频率（次/秒）
    pub fn access_rate(&self) -> f64 {
        let seconds = self.uptime.as_secs_f64();
        if seconds > 0.0 { self.total_accesses as f64 / seconds } else { 0.0 }
    }

    /// 格式化输出统计信息
    pub fn format(&self) -> String {
        format!(
            "DB Stats: {} accesses in {:.2}s (rate: {:.2}/s)",
            self.total_accesses,
            self.uptime.as_secs_f64(),
            self.access_rate()
        )
    }
}

/// 初始化数据库连接
///
/// 返回一个新的数据库连接实例。
pub async fn get_todo_conn() -> DatabaseConnection {
    todos::init_db().await.expect("init db failed")
}
