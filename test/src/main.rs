mod db_content;
mod db_test;
mod memory_leak;
mod model;
mod perf_benchmark;

// 主函数
fn main() {
    // 运行数据库连接测试
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        db_test::main().await.unwrap();
    });

    // 运行数据库内容检查
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        db_content::main().await.unwrap();
    });

    // 运行性能基准测试
    perf_benchmark::main();

    // 运行内存泄漏检测
    memory_leak::main();
}
