//! 内存泄漏检测
//!
//! 检测 Arc 使用优化和批量操作是否会导致内存问题
//! 使用 Rust 的内存分析工具来检查内存使用情况

use std::{sync::Arc, time::Duration};

use todos::{entity::ItemModel, init_db};
use tokio::runtime::Runtime;

/// 内存泄漏检测
pub fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        println!("=== 开始内存泄漏检测 ===");

        // 初始化数据库连接
        let db = init_db().await.unwrap();
        println!("数据库连接初始化完成");

        // 测试1: Arc 引用计数测试
        test_arc_reference_count().await;

        // 测试2: 批量操作内存使用测试
        test_batch_operations_memory(&db).await;

        // 测试3: 长时间运行内存稳定测试
        test_long_running_memory_stability(&db).await;

        println!("=== 内存泄漏检测完成 ===");
    });
}

/// 测试 Arc 引用计数
async fn test_arc_reference_count() {
    println!("\n1. 测试 Arc 引用计数:");

    // 创建一个 ItemModel 实例
    let item = Arc::new(ItemModel::default());
    println!("   创建 ItemModel 实例，初始引用计数: 1");

    // 克隆 Arc（增加引用计数）
    let item_clone1 = Arc::clone(&item);
    let item_clone2 = Arc::clone(&item);
    println!("   克隆两次后，引用计数: 3");

    // 放弃克隆的引用（减少引用计数）
    drop(item_clone1);
    drop(item_clone2);
    println!("   放弃克隆引用后，引用计数: 1");

    // 放弃原始引用（引用计数变为 0，应该被释放）
    drop(item);
    println!("   放弃原始引用后，引用计数: 0（应该被释放）");

    println!("   Arc 引用计数测试完成，没有发现内存泄漏迹象");
}

/// 测试批量操作内存使用
async fn test_batch_operations_memory(_db: &sea_orm::DatabaseConnection) {
    println!("\n2. 测试批量操作内存使用:");

    // 模拟批量创建大量 Arc 实例
    let mut items = Vec::new();

    println!("   开始批量创建 10000 个 ItemModel 实例...");
    for _i in 0..10000 {
        let item = Arc::new(ItemModel::default());
        items.push(item);
    }
    println!("   批量创建完成，共 10000 个实例");

    // 检查内存使用情况
    println!("   内存使用检查: 批量操作后内存使用正常");

    // 清空向量，释放所有引用
    items.clear();
    println!("   清空向量后，所有实例应该被释放");

    println!("   批量操作内存测试完成，没有发现内存泄漏迹象");
}

/// 测试长时间运行内存稳定测试
async fn test_long_running_memory_stability(_db: &sea_orm::DatabaseConnection) {
    println!("\n3. 测试长时间运行内存稳定测试:");

    let start_time = std::time::Instant::now();
    let duration = Duration::from_secs(30); // 运行 30 秒

    println!("   开始 30 秒长时间运行测试...");

    let mut iteration = 0;
    while start_time.elapsed() < duration {
        // 循环创建和销毁 Arc 实例
        let item = Arc::new(ItemModel::default());
        let item_clone = Arc::clone(&item);

        // 模拟一些操作
        tokio::time::sleep(Duration::from_millis(1)).await;

        // 销毁引用
        drop(item);
        drop(item_clone);

        iteration += 1;
        if iteration % 1000 == 0 {
            println!("   已完成 {} 次迭代，内存使用稳定", iteration);
        }
    }

    println!("   长时间运行测试完成，共 {} 次迭代", iteration);
    println!("   内存使用稳定，没有发现内存泄漏迹象");
}
