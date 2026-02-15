mod model;

use model::*;
use std::time::Instant;

// 主函数（tokio 异步运行时）
#[tokio::main]
async fn main() {
    // 包装 Job 为 Arc<Mutex<Job>>（线程安全的可变共享）
    let job = Job::new();
    // 调用不同并发方案
    let start = Instant::now();
    // job.get_logs_stream().await; // Tokio stream + buffer 696.1229ms
    // job.get_logs_with_semaphore(10).await;    // Tokio semaphore 449.8348ms
    // job.get_logs_smol_join().await;           // Smol 运行时 490.2271ms
    std::sync::Arc::new(job).get_logs_rayon_tokio_fixed().await; // Rayon + Tokio 混合 283.4552ms
    println!(" 耗时: {:?}", start.elapsed());
}
