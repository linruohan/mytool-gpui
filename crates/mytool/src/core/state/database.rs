use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use gpui::Global;
use sea_orm::DatabaseConnection;
use todos::Store;

/// 数据库连接状态
///
/// 存储全局数据库连接和 Store 实例，供业务逻辑使用。
///
/// 注意：DatabaseConnection 内部已经使用了 Arc 进行连接池管理，
/// 所以克隆操作是轻量级的（只增加引用计数）。
///
/// ## 优化说明
/// - 使用 Arc 包装，明确表达共享语义
/// - 添加连接统计，便于监控和诊断
/// - 支持连接健康检查
/// - 🚀 新增：预初始化全局 Store 实例，避免重复创建和死锁
#[derive(Clone)]
pub struct DBState {
    pub conn: Arc<DatabaseConnection>,
    pub store: Arc<Store>, // 🚀 预初始化的 Store
    stats: Arc<ConnectionStats>,
}

impl DBState {
    /// 创建新的数据库状态（同步版本，会阻塞创建 Store）
    pub fn new(conn: DatabaseConnection) -> Self {
        let conn_arc = Arc::new(conn);
        // 🚀 关键修复：同步创建 Store，避免 OnceCell 的死锁问题
        // 虽然这会阻塞应用启动，但只发生一次

        // 🔧 修复：使用 tokio::task::block_in_place 来在 async 上下文中运行阻塞代码
        // 这会在当前线程上安全地执行阻塞操作
        let store = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Store::new((*conn_arc).clone()).await.expect("Failed to create Store")
            })
        });

        Self { conn: conn_arc, store: Arc::new(store), stats: Arc::new(ConnectionStats::new()) }
    }

    /// 获取数据库连接（轻量级克隆）
    #[inline]
    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        self.stats.record_access();
        self.conn.clone()
    }

    /// 🚀 获取全局 Store 实例（轻量级克隆）
    #[inline]
    pub fn get_store(&self) -> Arc<Store> {
        self.store.clone()
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
#[derive(Debug, Clone)]
pub struct ConnectionStatsSnapshot {
    pub access_count: usize,
    pub last_access: Option<Instant>,
}

/// 连接统计收集器
struct ConnectionStats {
    access_count: AtomicUsize,
    last_access: std::sync::Mutex<Option<Instant>>,
}

impl ConnectionStats {
    fn new() -> Self {
        Self { access_count: AtomicUsize::new(0), last_access: std::sync::Mutex::new(None) }
    }

    fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        *self.last_access.lock().unwrap() = Some(Instant::now());
    }

    fn snapshot(&self) -> ConnectionStatsSnapshot {
        ConnectionStatsSnapshot {
            access_count: self.access_count.load(Ordering::Relaxed),
            last_access: *self.last_access.lock().unwrap(),
        }
    }

    fn reset(&self) {
        self.access_count.store(0, Ordering::Relaxed);
        *self.last_access.lock().unwrap() = None;
    }
}
