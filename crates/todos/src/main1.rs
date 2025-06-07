#![recursion_limit = "1024"]
#![allow(unused)]
use std::error::Error;
pub mod constants;
pub mod enums;
pub mod objects;
pub mod services;
use paste::paste;

pub mod utils;
use chrono::Datelike;
pub(crate) use objects::{
    Attachment, BaseObject, BaseTrait, Database, Item, Label, Project, Reminder, Section, Source,
    ToBool, schema,
};
pub(crate) use services::{Store, load_config};
pub(crate) use utils::Util;
#[macro_use]
extern crate paste;
fn main() -> Result<(), Box<dyn Error>> {
    // let db = Database::default();
    // db.get_sources_collection();

    let config = load_config()?;
    println!("Server: {}:{}", config.server.host, config.server.port);
    println!("Database URL: {}", config.database.url);
    println!("Database Pool Size: {}", config.database.pool_size);
    Ok(())
}

// use std::thread;

// fn main() {
//     // 初始化连接池并用 Arc 包装
//     let pool: DbPool = init_pool();

//     // 克隆 Arc 以共享连接池
//     let pool_clone = Arc::clone(&pool);

//     // 在另一个线程中插入用户
//     let handle = thread::spawn(move || {
//         let new_user = NewUser {
//             name: "Bob".to_string(),
//             email: "bob@example.com".to_string(),
//         };
//         insert_user(&pool_clone, new_user).expect("Failed to insert user");
//     });

//     // 在主线程中查询用户
//     let users = get_all_users(&pool).expect("Failed to load users");
//     for user in users {
//         println!("User: {} ({})", user.name, user.email);
//     }

//     // 等待子线程完成
//     handle.join().unwrap();
// }
