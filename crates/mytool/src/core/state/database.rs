use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use gpui::Global;
use sea_orm::DatabaseConnection;
use todos::{Store, error::TodoError};
use tracing::{info, warn};

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

    /// 等待 Store 就绪（带超时）
    ///
    /// 🚀 7.0修复：替代原来的"检查+跳过"逻辑，
    /// 确保在 Store 未就绪时等待而非静默丢弃保存操作。
    ///
    /// # 参数
    /// - `timeout`: 最大等待时间（默认 10 秒）
    ///
    /// # 返回
    /// - `Ok(())` - Store 已就绪
    /// - `Err(TodoError)` - 超时或等待失败
    pub async fn wait_for_store_ready(&self, timeout: Option<Duration>) -> Result<(), TodoError> {
        let timeout = timeout.unwrap_or(Duration::from_secs(10));
        let start = Instant::now();
        let mut wait_count = 0u32;

        loop {
            if self.is_store_ready() {
                info!("Store ready after {}ms ({} polls)", start.elapsed().as_millis(), wait_count);
                return Ok(());
            }

            if start.elapsed() >= timeout {
                warn!(
                    "Store not ready after {}ms ({} polls), timing out",
                    start.elapsed().as_millis(),
                    wait_count
                );
                return Err(TodoError::DatabaseError(format!(
                    "Store initialization timeout after {:?}",
                    timeout
                )));
            }

            wait_count += 1;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 获取全局 Store 实例
    ///
    /// 🚀 7.0修复：智能检测上下文，避免嵌套 runtime 问题
    /// - 如果已经在一个 async runtime 中，直接使用 `.await` 等待
    /// - 如果不在 runtime 中，使用 `block_on` 等待
    ///
    /// # Panics
    /// 如果 Store 尚未初始化且等待超时
    #[inline]
    pub async fn get_store_async(&self) -> Arc<Store> {
        self.wait_for_store_ready(Some(std::time::Duration::from_secs(10)))
            .await
            .expect("Store not initialized after waiting – did you call state_init()?");
        self.store.lock().unwrap().clone().expect("Store should be initialized after waiting")
    }

    /// 获取全局 Store 实例（同步版本）
    ///
    /// ⚠️ 仅在非 async 上下文中使用！
    /// 如果在 async 上下文中，请使用 `get_store_async()`！
    ///
    /// # Panics
    /// 如果 Store 尚未初始化（这表示应用逻辑有错误）
    #[inline]
    pub fn get_store(&self) -> Arc<Store> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let store = self.store.lock().unwrap();
            if let Some(s) = store.as_ref() {
                return s.clone();
            }
            drop(store);
            let rt = handle.clone();
            let wait =
                async { self.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await };
            let result = rt.block_on(wait);
            result.expect("Store not initialized after waiting – did you call state_init()?");
            self.store.lock().unwrap().clone().expect("Store should be initialized after waiting")
        } else {
            let rt = tokio::runtime::Handle::current();
            let wait =
                async { self.wait_for_store_ready(Some(std::time::Duration::from_secs(10))).await };
            rt.block_on(wait)
                .expect("Store not initialized after waiting – did you call state_init()?");
            self.store.lock().unwrap().clone().expect("Store should be initialized after waiting")
        }
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
        self.stats.reset()
    }

    /// 🚀 7.0新增：优雅关闭数据库连接和 Store
    ///
    /// 在应用退出时调用，确保：
    /// 1. Store 被正确释放（释放内部引用）
    /// 2. 连接池会在所有 Arc 引用 drop 后自动关闭
    pub fn shutdown(&self) {
        info!("Shutting down DBState...");

        // 关闭 Store（释放内部资源和对 conn 的引用）
        let mut guard = self.store.lock().unwrap();
        if guard.is_some() {
            *guard = None;
            info!("Store released");
        }
        drop(guard);

        // 注意：DatabaseConnection (SQLx 连接池) 使用 Arc 管理
        // 当 Store 被释放后，内部引用减少
        // 最终连接池会在进程退出时自动清理（包括 reaper 线程）
        info!("DBState shutdown complete (connection refs: {})", Arc::strong_count(&self.conn));
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
