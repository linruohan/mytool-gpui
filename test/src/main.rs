mod memory_leak;
mod model;
mod perf_benchmark;

// 主函数
fn main() {
    // 运行性能基准测试
    perf_benchmark::main();

    // 运行内存泄漏检测
    memory_leak::main();
}
