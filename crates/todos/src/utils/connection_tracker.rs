//! 连接追踪器
//!
//! 用于监控 Sea-ORM 连接池的连接获取/释放，
//! 帮助定位 ConnectionAcquire(Timeout) 的根因。

use std::sync::{Arc, Mutex};

use tracing::{info, warn};

/// 连接使用记录
#[derive(Debug, Clone)]
pub struct ConnectionRecord {
    /// 获取时间
    pub acquired_at: std::time::Instant,
    /// 调用者标识
    pub caller: String,
    /// 线程 ID
    pub thread_id: std::thread::ThreadId,
    /// 是否已释放
    pub released: bool,
    /// 释放时间
    pub released_at: Option<std::time::Instant>,
    /// 持续时间（毫秒）
    pub duration_ms: u64,
}

impl ConnectionRecord {
    pub fn new(caller: &str) -> Self {
        Self {
            acquired_at: std::time::Instant::now(),
            caller: caller.to_string(),
            thread_id: std::thread::current().id(),
            released: false,
            released_at: None,
            duration_ms: 0,
        }
    }

    /// 标记为已释放
    pub fn release(&mut self) {
        self.released = true;
        self.released_at = Some(std::time::Instant::now());
        self.duration_ms =
            self.released_at.unwrap().duration_since(self.acquired_at).as_millis() as u64;
    }
}

/// 全局连接追踪器
static CONNECTION_TRACKER: std::sync::OnceLock<Arc<Mutex<Vec<ConnectionRecord>>>> =
    std::sync::OnceLock::new();

/// 获取连接追踪器实例
fn get_tracker() -> Arc<Mutex<Vec<ConnectionRecord>>> {
    CONNECTION_TRACKER.get_or_init(|| Arc::new(Mutex::new(Vec::new()))).clone()
}

/// 记录一次连接获取
///
/// # 参数
/// - `caller`: 调用者标识（用于定位代码位置）
///
/// # 返回
/// 连接记录的索引（用于后续释放时匹配）
pub fn track_acquire(caller: &str) -> usize {
    let tracker = get_tracker();
    let mut records = tracker.lock().unwrap();

    let record = ConnectionRecord::new(caller);
    let index = records.len();
    records.push(record);

    // 打印获取日志
    let active_count = records.iter().filter(|r| !r.released).count();
    info!(
        "🔗 [ConnTracker] 获取连接 #{} (caller='{}', 活跃连接数={})",
        index, caller, active_count
    );

    // 如果活跃连接数超过阈值，打印警告
    if active_count > 3 {
        warn!("⚠️ [ConnTracker] 活跃连接数过高! {} (阈值=3), 可能存在连接泄漏!", active_count);
        print_active_connections(&records);
    }

    index
}

/// 记录一次连接释放
///
/// # 参数
/// - `index`: 连接记录的索引（由 track_acquire 返回）
pub fn track_release(index: usize) {
    let tracker = get_tracker();
    let mut records = tracker.lock().unwrap();

    if let Some(record) = records.get_mut(index) {
        record.release();

        info!(
            "🔗 [ConnTracker] 释放连接 #{} (caller='{}', 持续={}ms)",
            index, record.caller, record.duration_ms
        );

        // 如果持续时间过长，打印警告
        if record.duration_ms > 1000 {
            warn!(
                "⚠️ [ConnTracker] 连接 #{} 持续时间过长! {}ms (caller='{}')",
                index, record.duration_ms, record.caller
            );
        }
    } else {
        warn!("❌ [ConnTracker] 尝试释放不存在的连接 #{}", index);
    }
}

/// 打印所有活跃连接的状态
pub fn print_active_connections(records: &[ConnectionRecord]) {
    let active: Vec<&ConnectionRecord> = records.iter().filter(|r| !r.released).collect();

    if active.is_empty() {
        info!("✅ [ConnTracker] 无活跃连接");
        return;
    }

    warn!("📊 [ConnTracker] 当前活跃连接列表 (共 {} 个):", active.len());
    for (i, record) in active.iter().enumerate() {
        let elapsed = record.acquired_at.elapsed().as_millis();
        warn!(
            "  [{}] caller='{}', thread={:?}, 已持有了 {}ms",
            i + 1,
            record.caller,
            record.thread_id,
            elapsed
        );
    }
}

/// 打印当前所有连接的摘要信息
pub fn print_connection_summary() {
    let tracker = get_tracker();
    let records = tracker.lock().unwrap();

    let total = records.len();
    let active = records.iter().filter(|r| !r.released).count();
    let released = total - active;

    info!("📊 [ConnTracker] 连接统计: 总计={}, 活跃={}, 已释放={}", total, active, released);

    if active > 0 {
        print_active_connections(&records);
    }
}

/// 清空所有记录（用于测试或重置）
pub fn clear_records() {
    let tracker = get_tracker();
    let mut records = tracker.lock().unwrap();
    records.clear();
}
