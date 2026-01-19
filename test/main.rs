mod model;
// mod to_hy;

use model::*;
use std::sync::Arc;

// 主函数（tokio 异步运行时）
#[tokio::main]
async fn main() {
    // 包装 Job 为 Arc<Mutex<Job>>（线程安全的可变共享）
    let job = Job::new();
    // 调用 get_logs 方法
    // job.get_logs_stream().await;
    // job.get_logs_async_std().await;
    // job.get_logs_with_semaphore(10).await;
    // job.get_logs_smol_join().await;
    // job.get_logs_smol_stream().await;
    // Arc::new(job).get_logs_rayon_tokio_correct().await; // 对了，但是主线程退出了
    // Arc::new(job).get_logs_with_existing_runtime().await;
    // Arc::new(job).get_logs_with_handle().await;
    // Arc::new(job).get_logs_block_in_place().await;
    Arc::new(job).get_logs_rayon_tokio_fixed().await;
}
