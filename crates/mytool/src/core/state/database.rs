use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use gpui::Global;
use sea_orm::DatabaseConnection;
use todos::{Store, error::TodoError};

/// 数据库连接状态
///
/// 存储全局数据库连接和 Store 实例，供业务逻辑使用。
///
/// 注意：DatabaseConnection 内部已经使用了 Arc 进行连接池管理，
/// 所以克隆操作是轻量级的（只增加引用计数）。
///
/// ## 优化说明
/// - 🚀 6.1优化：Store 异步创建，不再阻塞首帧
/// - 使用 Arc 包装，明确表达共享语义
/// - 添加连接统计，便于监控和诊断
/// - 支持连接健康检查
#[derive(Clone)]
pub struct DBState {
    pub conn: Arc<DatabaseConnection>,
    store: Arc<Mutex<Option<Arc<Store>>>>, // 🚀 6.1: 异步初始化
    stats: Arc<ConnectionStats>,
}

impl DBState {
    /// 创建新的数据库状态（不阻塞创建 Store）
    ///
    /// 🚀 6.1优化：Store 将在 state_init 的异步任务中创建，
    /// 避免阻塞应用首帧。
    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn: Arc::new(conn),
            store: Arc::new(Mutex::new(None)),
            stats: Arc::new(ConnectionStats::new()),
        }
    }

    /// 异步创建并设置 Store 实例
    ///
    /// 此方法应在 state_init 的 spawn 任务中调用，
    /// 不阻塞主线程首帧渲染。
    pub async fn init_store(&self) -> Result<Arc<Store>, TodoError> {
        let conn = (*self.conn).clone();
        let store = Store::new(conn).await?;
        let store_arc = Arc::new(store);

        let mut guard = self.store.lock().unwrap();
        *guard = Some(store_arc.clone());

        Ok(store_arc)
    }

    /// 检查 Store 是否已初始化
    #[inline]
    pub fn is_store_ready(&self) -> bool {
        self.store.lock().unwrap().is_some()
    }

    /// 获取全局 Store 实例
    ///
    /// 🚀 6.1优化：Store 通过异步创建，此方法在 Store 未就绪时会 panic
    /// （这表明存在编程错误：Store 初始化前的操作）
    ///
    /// # Panics
    /// 如果 Store 尚未初始化（这表示应用逻辑有错误）
    #[inline]
    pub fn get_store(&self) -> Arc<Store> {
        self.store
            .lock()
            .unwrap()
            .clone()
            .expect("Store not initialized - did you call state_init()?")
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
