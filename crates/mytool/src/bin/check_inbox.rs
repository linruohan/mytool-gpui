use chrono;
use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel};

#[tokio::main]
async fn main() {
    println!("Checking inbox data...");

    // 获取数据库连接
    println!("Initializing database connection...");
    let db = todos::init_db().await.expect("Failed to initialize database");
    println!("Database connection established.");

    // 创建 Store 实例
    let store = Store::new(db.clone());

    // 检查所有未完成的任务
    println!("\n=== All incomplete tasks ===");
    let incomplete_items = store.get_incomplete_items().await;
    match incomplete_items {
        Ok(items) => {
            println!("Found {} incomplete tasks:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}. ID: {}", i + 1, item.id);
                println!("   Content: {}", item.content);
                println!("   Checked: {}", item.checked);
                println!("   Project ID: {:?}", item.project_id);
                println!("   Section ID: {:?}", item.section_id);
                println!("   Pinned: {}", item.pinned);
                println!();
            }

            // 检查 inbox 条件的任务
            println!("\n=== Inbox tasks (no project ID) ===");
            let inbox_items: Vec<&ItemModel> = items
                .iter()
                .filter(|item| item.project_id.is_none() || item.project_id.as_deref() == Some(""))
                .collect();
            println!("Found {} inbox tasks:", inbox_items.len());
            for (i, item) in inbox_items.iter().enumerate() {
                println!("{}. ID: {}", i + 1, item.id);
                println!("   Content: {}", item.content);
                println!("   Project ID: {:?}", item.project_id);
                println!();
            }

            // 如果没有 inbox 任务，添加一个测试任务
            if inbox_items.is_empty() {
                println!("\n=== Adding test task to inbox ===");
                let test_item = ItemModel {
                    id: format!("test-task-{}", chrono::Utc::now().timestamp()),
                    content: "Test inbox task".to_string(),
                    checked: false,
                    project_id: None, // 无项目 ID，应该显示在 inbox 中
                    section_id: None,
                    pinned: false,
                    ..Default::default()
                };

                match store.insert_item(test_item, true).await {
                    Ok(new_item) => {
                        println!("Added test task: {}", new_item.content);
                        println!("Task ID: {}", new_item.id);
                        println!("Project ID: {:?}", new_item.project_id);
                    },
                    Err(e) => {
                        println!("Error adding test task: {:?}", e);
                    },
                }
            }
        },
        Err(e) => {
            println!("Error getting incomplete items: {:?}", e);
        },
    }

    // 检查所有任务（包括已完成的）
    println!("\n=== All completed tasks ===");
    let completed_items = store.get_completed_items().await;
    match completed_items {
        Ok(items) => {
            println!("Found {} completed tasks:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}. ID: {}", i + 1, item.id);
                println!("   Content: {}", item.content);
                println!("   Checked: {}", item.checked);
                println!();
            }
        },
        Err(e) => {
            println!("Error getting completed items: {:?}", e);
        },
    }

    println!("\nCheck completed!");
}
