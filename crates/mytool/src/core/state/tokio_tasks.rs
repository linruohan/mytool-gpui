//! Tokio 任务追踪器
//!
//! 用于追踪所有 tokio 异步任务，确保应用退出前所有任务完成。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use gpui::Global;
use tokio::task::JoinHandle;

/// Tokio 任务追踪器
#[derive(Clone)]
pub struct TokioTasksTracker {
    inner: Arc<TokioTasksTrackerInner>,
}

struct TokioTasksTrackerInner {
    /// 正在进行的任务数量
    pending_count: AtomicUsize,
    /// 任务句柄列表
    handles: Mutex<Vec<JoinHandle<()>>>,
    /// 是否正在关闭
    shutting_down: AtomicBool,
}

impl Default for TokioTasksTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioTasksTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TokioTasksTrackerInner {
                pending_count: AtomicUsize::new(0),
                handles: Mutex::new(Vec::new()),
                shutting_down: AtomicBool::new(false),
            }),
        }
    }

    /// 添加一个任务
    pub fn add_task(&self, handle: JoinHandle<()>) {
        self.inner.pending_count.fetch_add(1, Ordering::SeqCst);
        self.inner.handles.lock().unwrap().push(handle);
    }

    /// 任务完成
    pub fn task_completed(&self) {
        self.inner.pending_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// 获取待处理任务数量
    pub fn pending_count(&self) -> usize {
        self.inner.pending_count.load(Ordering::SeqCst)
    }

    /// 是否正在关闭
    pub fn is_shutting_down(&self) -> bool {
        self.inner.shutting_down.load(Ordering::SeqCst)
    }

    /// 开始关闭
    pub fn start_shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::SeqCst);
    }

    /// 等待所有任务完成（阻塞）
    pub fn wait_all(&self, timeout: std::time::Duration) -> usize {
        self.start_shutdown();

        let start = std::time::Instant::now();

        loop {
            let remaining = self.inner.pending_count.load(Ordering::SeqCst);
            if remaining == 0 {
                tracing::info!("✅ All tokio tasks completed");
                return 0;
            }

            if start.elapsed() >= timeout {
                tracing::warn!("⚠️ Timeout waiting for {} tokio tasks", remaining);
                return remaining;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

impl Global for TokioTasksTracker {}
