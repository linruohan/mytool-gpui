#![recursion_limit = "1024"]
#![allow(unused)]
use std::error::Error;
pub mod constants;
pub mod enums;
pub mod services;
pub mod settings;
use paste::paste;
mod app;
mod entity;
mod filters;
mod objects;

pub mod utils;
use chrono::Datelike;
pub(crate) use objects::{
    Attachment, BaseObject, BaseTrait, Item, Label, Project, Reminder, Section, Source, ToBool,
};
pub(crate) use services::Store;
pub(crate) use utils::Util;
#[macro_use]
extern crate paste;

pub async fn init() -> anyhow::Result<()> {
    app::init().await
}
