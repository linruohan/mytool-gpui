use sea_orm::{Database, DbErr, Statement};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    println!("=== 检查数据库内容 ===");

    // 连接数据库
    let db = Database::connect("sqlite://db.sqlite?mode=rwc").await?;
    println!("数据库连接成功");

    // 检查所有表
    println!("\n1. 检查所有表:");
    let tables = db
        .query_all(Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        ))
        .await?;

    if tables.is_empty() {
        println!("   没有找到表");
    } else {
        println!("   找到 {} 个表:", tables.len());
        for table in &tables {
            let name: String = table.try_get(0)?;
            println!("   - {}", name);
        }
    }

    // 检查每个表的记录数
    println!("\n2. 检查每个表的记录数:");
    for table in &tables {
        let name: String = table.try_get(0)?;
        let count = db
            .query_one(Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                format!("SELECT COUNT(*) FROM {}", name),
            ))
            .await?;

        if let Some(row) = count {
            let count: i64 = row.try_get(0)?;
            println!("   {}: {} 条记录", name, count);
        }
    }

    // 检查db_version表
    println!("\n3. 检查数据库版本:");
    let version = db
        .query_one(Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT version FROM db_version ORDER BY version DESC LIMIT 1",
        ))
        .await;

    match version {
        Ok(Some(row)) => {
            let version: i32 = row.try_get(0)?;
            println!("   当前数据库版本: {}", version);
        },
        Ok(None) => {
            println!("   数据库版本表为空");
        },
        Err(e) => {
            println!("   检查数据库版本失败: {:?}", e);
        },
    }

    // 检查items表的前几条记录
    println!("\n4. 检查items表的前5条记录:");
    let items = db
        .query_all(Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT id, title, status FROM items LIMIT 5",
        ))
        .await;

    match items {
        Ok(rows) => {
            if rows.is_empty() {
                println!("   items表为空");
            } else {
                println!("   items表前5条记录:");
                for row in &rows {
                    let id: String = row.try_get(0)?;
                    let title: String = row.try_get(1)?;
                    let status: String = row.try_get(2)?;
                    println!("   - {}: {} (status: {})");
                }
            }
        },
        Err(e) => {
            println!("   检查items表失败: {:?}", e);
        },
    }

    println!("\n=== 检查完成 ===");
    Ok(())
}
