#![recursion_limit = "1024"]
#![allow(unused)]
#[macro_use]
extern crate paste;
use std::error::Error;
mod app;
pub mod constants;
pub mod entity;
pub mod enums;
mod filters;
pub mod objects;
pub mod services;
pub mod settings;

pub mod utils;

pub use app::init_db;
use chrono::Datelike;
pub(crate) use objects::{
    Attachment, BaseObject, BaseTrait, Item, Label, Project, Reminder, Section, Source, ToBool,
};
pub(crate) use services::Store;
pub(crate) use utils::Util;
