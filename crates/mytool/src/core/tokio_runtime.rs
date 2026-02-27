//! 独立的 Tokio Runtime 用于数据库操作
//!
//! 由于 GPUI 运行在自己的事件循环中，而 Sea-ORM 需要 tokio runtime，
//! 我们创建一个独立的 tokio runtime 来执行数据库操作，
//! 避免与主 tokio runtime 的生命周期冲突。

use std::sync::OnceLock;

use tokio::runtime::Runtime;

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
pub fn run_db_operation<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let runtime = get_db_runtime();
    runtime.block_on(future)
}

/// 在独立的 tokio runtime 中执行异步操作（非阻塞，返回 JoinHandle）
pub fn spawn_db_operation<F, T>(future: F) -> tokio::task::JoinHandle<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let runtime = get_db_runtime();
    runtime.spawn(future)
}
