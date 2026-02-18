use sea_orm::DatabaseConnection;
use todos::{Store, entity::ItemModel};
use chrono;

#[tokio::main]
async fn main() {
    println!("Adding today task...");
    
    // 获取数据库连接
    println!("Initializing database connection...");
    let db = todos::init_db().await.expect("Failed to initialize database");
    println!("Database connection established.");
    
    // 创建 Store 实例
    let store = Store::new(db.clone());
    
    // 创建一个今天到期的任务
    println!("\n=== Adding today task ===");
    
    // 获取今天的 UTC 日期时间字符串 (YYYY-MM-DDTHH:MM:SS 格式)
    let today = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    println!("Today's UTC date time: {}", today);
    
    // 创建完整的 DueDate 结构
    let due_date = todos::DueDate {
        date: today,
        timezone: "UTC".to_string(),
        recurrency_weeks: "".to_string(),
        is_recurring: false,
        recurrency_type: todos::enums::RecurrencyType::NONE,
        recurrency_interval: 0,
        recurrency_count: 0,
        recurrency_end: "".to_string(),
        recurrency_supported: false,
    };
    
    let due_json = serde_json::to_value(due_date).unwrap();
    
    let today_task = ItemModel {
        id: format!("today-task-{}", chrono::Utc::now().timestamp()),
        content: format!("Today's task - must be done today! {}", chrono::Utc::now().timestamp()).to_string(),
        checked: false,
        project_id: None,
        section_id: None,
        pinned: false,
        due: Some(due_json),
        ..Default::default()
    };
    
    match store.insert_item(today_task, true).await {
        Ok(new_item) => {
            println!("Added today task: {}", new_item.content);
            println!("Task ID: {}", new_item.id);
            println!("Due date: {:?}", new_item.due);
            println!("Project ID: {:?}", new_item.project_id);
        },
        Err(e) => {
            println!("Error adding today task: {:?}", e);
        }
    }
    
    // 检查 today 任务是否正确加载
    println!("\n=== Checking today tasks ===");
    let incomplete_items = store.get_incomplete_items().await;
    match incomplete_items {
        Ok(items) => {
            println!("Found {} incomplete tasks:", items.len());
            
            // 获取今天的 UTC 日期
            let today_utc = chrono::Utc::now().naive_utc().date();
            println!("Today's UTC date: {:?}", today_utc);
            
            // 检查 today 条件的任务
            println!("\n=== Today tasks (due today) ===");
            let today_items: Vec<&ItemModel> = items
                .iter()
                .filter(|item| {
                    // 添加调试信息
                    if let Some(due_date) = item.due_date() {
                        if let Some(due_datetime) = due_date.datetime() {
                            println!("Debug: Item {} due date: {:?}, today: {:?}, equal: {}", 
                                item.content, due_datetime.date(), today_utc, due_datetime.date() == today_utc);
                        } else {
                            println!("Debug: Item {} due date parsing failed", item.content);
                        }
                    } else {
                        println!("Debug: Item {} has no due date", item.content);
                    }
                    item.is_due_today()
                })
                .collect();
            println!("Found {} today tasks:", today_items.len());
            
            for (i, item) in today_items.iter().enumerate() {
                println!("{}. ID: {}", i+1, item.id);
                println!("   Content: {}", item.content);
                println!("   Due date: {:?}", item.due);
                println!();
            }
        },
        Err(e) => {
            println!("Error getting incomplete items: {:?}", e);
        }
    }
    
    println!("\nTask added successfully!");
}
