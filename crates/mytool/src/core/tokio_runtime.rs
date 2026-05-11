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

use std::sync::OnceLock;

use tokio::runtime::{Handle, Runtime};

/// 全局独立的 Tokio Runtime（专门用于数据库操作）
static DB_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// 获取或创建数据库专用的 Tokio Runtime
fn get_db_runtime() -> &'static Runtime {
    DB_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("db-worker")
            .enable_all()
            .build()
            .expect("Failed to create database runtime")
    })
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
        // 🚀 6.2优化：在现有 runtime 中，使用 spawn + block_on 等待
        // 避免 std::thread::spawn 的线程创建开销
        let runtime = get_db_runtime();
        let handle = runtime.spawn(future);
        // 在当前 runtime 中 block_on 等待 DB runtime 的任务完成
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(handle))
            .expect("DB operation task panicked")
    } else {
        // 不在 runtime 中：直接使用独立 runtime
        let runtime = get_db_runtime();
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
    let runtime = get_db_runtime();
    runtime.spawn(future)
}
