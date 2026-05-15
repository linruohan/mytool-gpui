#![recursion_limit = "1024"]
// 🚀 6.10优化：收紧 allow(unused)，改为模块级控制
// 之前使用 #![allow(unused)] 会掩盖未使用 API 与 死代码
// 现在只在特定模块允许 unused，便于发现死代码
#![allow(unused_imports)] // 允许未使用导入（避免重构时频繁修改）
#![allow(dead_code)] // 允许死代码（filters/objects 等模块为未来功能预留）
#[macro_use]
extern crate paste;
mod app;
pub mod constants;
pub mod entity;
pub mod enums;
pub mod error;
mod filters;
mod objects;
pub mod repositories;
pub mod services;
pub mod utils;

pub use app::init_db;
use chrono::Datelike;
pub use objects::due_date::DueDate;
pub(crate) use objects::{BaseObject, BaseTrait, Item, Reminder, ToBool};
pub use services::Store;
pub(crate) use utils::Util;
