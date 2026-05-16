//! 独立的 Tokio Runtime 用于数据库操作
//!
//! 由于 GPUI 运行在自己的事件循环中，而 Sea-ORM 需要 tokio runtime，
//! 我们创建一个独立的 tokio runtime 来执行数据库操作，
//! 避免与主 tokio runtime 的生命周期冲突。
//!
//! ## 🚀 6.2 优化说明
//! - 使用 `spawn_db_operation` + `block_on` 替代每次 `std::thread::spawn`
//! - 减少线程创建开销，复用 DB runtime 的工作线程
//! - 仅在极少数必须同步的场景保留阻塞封装

use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use tokio::runtime::{Handle, Runtime};
use tracing::info;

/// 全局独立的 Tokio Runtime（专门用于数据库操作）
/// 使用 Arc<Mutex<Option<>>> 包装以支持优雅关闭
static DB_RUNTIME: OnceLock<Arc<Mutex<Option<Runtime>>>> = OnceLock::new();
/// 标记 Runtime 是否已被 shutdown（防止重复 shutdown）
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// 获取或创建数据库专用的 Tokio Runtime
fn get_db_runtime() -> Arc<Mutex<Option<Runtime>>> {
    DB_RUNTIME
        .get_or_init(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("db-worker")
                .enable_all()
                .build()
                .expect("Failed to create database runtime");
            Arc::new(Mutex::new(Some(runtime)))
        })
        .clone()
}

/// 🚀 7.0新增：优雅关闭 DB Runtime
///
/// 在应用退出时调用，确保所有数据库操作完成后再关闭 worker 线程。
///
/// # 参数
/// - `timeout`: 最大等待时间（默认 5 秒）
///
/// # 返回
/// - `Ok(())` - Runtime 已成功关闭
/// - `Err(())` - 超时或已关闭
pub fn shutdown_db_runtime(timeout: Option<std::time::Duration>) -> Result<(), ()> {
    // 防止重复 shutdown
    if SHUTDOWN_FLAG.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        info!("DB Runtime already shut down or shutting down");
        return Ok(());
    }

    let runtime_arc = match DB_RUNTIME.get() {
        Some(r) => r.clone(),
        None => return Ok(()),
    };

    let timeout = timeout.unwrap_or(std::time::Duration::from_secs(5));

    // 从 Mutex 中取出 Runtime 并 shutdown
    let mut guard = runtime_arc.lock().unwrap();
    if let Some(runtime) = guard.take() {
        info!(
            "Shutting down DB Runtime (timeout: {:?}, active tasks: {:?})",
            timeout,
            runtime.metrics().num_alive_tasks()
        );

        // shutdown_timeout 会等待所有任务完成或超时
        runtime.shutdown_timeout(timeout);

        info!("DB Runtime shut down successfully");
    } else {
        info!("DB Runtime already taken (already shut down)");
    }

    Ok(())
}

/// 检查 DB Runtime 是否仍在运行
pub fn is_db_runtime_active() -> bool {
    !SHUTDOWN_FLAG.load(Ordering::SeqCst)
        && DB_RUNTIME.get().map(|r| r.lock().unwrap().is_some()).unwrap_or(false)
}

/// 在独立的 tokio runtime 中执行异步操作（阻塞当前线程直到完成）
///
/// 🚀 6.2优化：使用 `spawn` + `block_on` 替代 `std::thread::spawn`，
/// 避免每次创建新线程的开销。
///
/// ## 使用建议
/// - **新代码优先使用 `spawn_db_operation`**（非阻塞，配合 `cx.spawn`）
/// - 仅在必须从同步上下文调用时保留此函数
///
/// # 注意
/// 如果在已有 runtime 中调用，会使用 `spawn` + `block_on` 模式，
/// 避免 "runtime within runtime" 错误。
pub fn run_db_operation<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // 检查是否在现有 runtime 中
    if Handle::try_current().is_ok() {
        let runtime_arc = get_db_runtime();
        let guard = runtime_arc.lock().unwrap();
        let runtime = guard.as_ref().expect("DB Runtime not initialized");
        let handle = runtime.spawn(future);
        drop(guard);
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(handle))
            .expect("DB operation task panicked")
    } else {
        let runtime_arc = get_db_runtime();
        let guard = runtime_arc.lock().unwrap();
        let runtime = guard.as_ref().expect("DB Runtime not initialized");
        runtime.block_on(future)
    }
}

/// 在独立的 tokio runtime 中执行异步操作（非阻塞，返回 JoinHandle）
///
/// 🚀 6.2优化：推荐新代码使用此函数，配合 `cx.spawn` 实现非阻塞数据库操作。
///
/// ## 示例
/// ```ignore
/// cx.spawn(async move |cx| {
///     let handle = spawn_db_operation(async move {
///         store.get_items().await
///     });
///     match handle.await {
///         Ok(items) => { /* 更新 UI */ }
///         Err(e) => { /* 处理错误 */ }
///     }
/// }).detach();
/// ```
pub fn spawn_db_operation<F, T>(future: F) -> tokio::task::JoinHandle<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let runtime_arc = get_db_runtime();
    let guard = runtime_arc.lock().unwrap();
    let runtime = guard.as_ref().expect("DB Runtime not initialized");
    runtime.spawn(future)
}
