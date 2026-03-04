use gconfig::{DatabaseConfig, get};
use sea_orm::{Database, DbErr};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    println!("=== 测试数据库连接 ===");

    // 1. 检查配置加载
    println!("1. 加载配置...");
    let config = get().read().unwrap();
    let db_config = config.database();

    println!("   数据库类型: {}", db_config.db_type());
    println!("   SQLite 路径: {}", db_config.sqlite_path());

    // 2. 检查路径解析
    println!("2. 解析数据库路径...");
    let resolved_path = resolve_db_path(db_config.sqlite_path());
    println!("   解析后的路径: {}", resolved_path);

    // 3. 检查目录是否存在
    println!("3. 检查目录是否存在...");
    let path = std::path::Path::new(&resolved_path);
    if let Some(parent) = path.parent() {
        if parent.exists() {
            println!("   目录存在: {}", parent.display());
        } else {
            println!("   目录不存在: {}", parent.display());
            // 尝试创建目录
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("   创建目录失败: {:?}", e);
            } else {
                println!("   目录创建成功");
            }
        }
    }

    // 4. 尝试连接数据库
    println!("4. 尝试连接数据库...");
    let url = format!("sqlite://{}?mode=rwc", resolved_path);
    println!("   连接 URL: {}", url);

    match Database::connect(url).await {
        Ok(db) => {
            println!("   连接成功!");
            // 尝试执行简单查询
            match db.ping().await {
                Ok(_) => println!("   数据库 ping 成功"),
                Err(e) => println!("   数据库 ping 失败: {:?}", e),
            }
        },
        Err(e) => {
            println!("   连接失败: {:?}", e);
        },
    }

    println!("=== 测试完成 ===");
    Ok(())
}

fn resolve_db_path(path: &str) -> String {
    use std::path::Path;

    let path_obj = Path::new(path);

    if path_obj.is_absolute() {
        path.to_string()
    } else {
        let mut base_path = std::env::current_dir().expect("获取当前目录失败");

        while !base_path.join("crates").exists() {
            if let Some(parent) = base_path.parent() {
                base_path = parent.to_path_buf();
            } else {
                break;
            }
        }

        base_path.join(path).to_str().expect("转换路径失败").to_string()
    }
}
