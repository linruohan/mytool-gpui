use crate::utils::asset_str;
use config::Config;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{borrow::Cow, error::Error};
#[derive(Debug, Deserialize, RustEmbed)]
#[folder = "assets"]
#[include = "settings/*"]
#[exclude = "*.DS_Store"]
pub struct SettingsAssets;
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: u32,
}
pub fn default_settings() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/settings.json")
}

pub fn load_config() -> Result<AppConfig, Box<dyn Error>> {
    serde_json::from_str::<AppConfig>(&default_settings()).map_err(Into::into)
}
