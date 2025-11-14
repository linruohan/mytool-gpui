use std::sync::LazyLock;

use anyhow::{Context, anyhow};
use config::{Config, FileFormat};
use serde::Deserialize;

mod database;
mod server;
use std::{borrow::Cow, error::Error};

use database::DatabaseConfig;
use rust_embed::RustEmbed;
use server::ServerConfig;

use crate::utils::asset_str;

static CONFIG: LazyLock<AppConfig> =
    LazyLock::new(|| AppConfig::load().expect("Failed to initialize app_config"));
#[derive(Deserialize, Debug)]
pub struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}
impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        Config::builder()
            .add_source(
                config::File::with_name("application").format(FileFormat::Toml).required(true),
            )
            .add_source(
                config::Environment::with_prefix("APP")
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(","),
            )
            .build()
            .with_context(|| anyhow!("failed to load app_config"))?
            .try_deserialize()
            .with_context(|| anyhow!("failed to deserialize app_config"))
    }

    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }
}
pub fn get() -> &'static AppConfig {
    &CONFIG
}
