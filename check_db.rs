use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel};

#[tokio::main]
async fn main() {
    println!("Checking database tasks...");
    
    // 获取数据库连接
    let db = get_todo_conn().await;
    println!("Database connection established.");
    
    // 创建 Store 实例
    let store = Store::new(db);
    
    // 检查所有未完成的任务
    println!("\n=== All incomplete tasks ===");
    let incomplete_items = store.get_incomplete_items().await;
    match incomplete_items {
        Ok(items) => {
            println!("Found {} incomplete tasks:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}. ID: {}", i+1, item.id);
                println!("   Title: {}", item.title);
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
                println!("{}. ID: {}", i+1, item.id);
                println!("   Title: {}", item.title);
                println!("   Project ID: {:?}", item.project_id);
                println!();
            }
        },
        Err(e) => {
            println!("Error getting incomplete items: {:?}", e);
        }
    }
    
    // 检查所有任务（包括已完成的）
    println!("\n=== All tasks (including completed) ===");
    let completed_items = store.get_completed_items().await;
    match completed_items {
        Ok(items) => {
            println!("Found {} completed tasks:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}. ID: {}, Title: {}, Checked: {}", i+1, item.id, item.title, item.checked);
                println!("   Project ID: {:?}", item.project_id);
                println!();
            }
        },
        Err(e) => {
            println!("Error getting completed items: {:?}", e);
        }
    }
}

// 从 mytool 中复制数据库连接函数
async fn get_todo_conn() -> DatabaseConnection {
    use sea_orm::Database;
    use std::path::Path;
    
    let db_path = Path::new("data").join("todo.db");
    let db_url = format!("sqlite:{}", db_path.display());
    
    Database::connect(db_url).await.unwrap()
}