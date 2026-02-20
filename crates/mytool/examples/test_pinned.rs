use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel};

#[tokio::main]
async fn main() {
    // 连接数据库
    let db = sea_orm::Database::connect("sqlite://db.sqlite?mode=rwc").await.unwrap();

    // 获取所有项目
    let items = Store::new(db.clone()).get_all_items().await.unwrap();
    println!("Loaded {} items", items.len());

    // 打印每个项目的pinned状态
    for item in &items {
        println!("Item {}: pinned = {}", item.id, item.pinned);
    }

    // 测试更新pinned状态
    let test_item_id = "test123";
    println!("\nTesting update_item_pin for item {}", test_item_id);

    // 尝试更新pinned状态
    let result = Store::new(db.clone()).update_item_pin(test_item_id, true).await;
    println!("Update result: {:?}", result);

    // 再次获取所有项目，查看pinned状态是否更新
    let items = Store::new(db.clone()).get_all_items().await.unwrap();
    println!("\nLoaded {} items after update", items.len());

    // 打印每个项目的pinned状态
    for item in &items {
        println!("Item {}: pinned = {}", item.id, item.pinned);
    }

    // 测试加载pinned项目
    let pinned_items = Store::new(db.clone()).get_pinned_items().await.unwrap();
    println!("\nLoaded {} pinned items", pinned_items.len());

    for item in &pinned_items {
        println!("Pinned item: {} - pinned = {}", item.id, item.pinned);
    }
}
