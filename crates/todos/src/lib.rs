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
    ToBool, filters, schema,
};
pub(crate) use services::{Store, load_config};
pub(crate) use utils::Util;
#[macro_use]
extern crate paste;
fn init() {
    // let db = Database::default();
    // db.get_sources_collection();

    let config = load_config().expect("failed get config");
    println!("Server: {}:{}", config.server.host, config.server.port);
    println!("Database URL: {}", config.database.url);
    println!("Database Pool Size: {}", config.database.pool_size);
}
