//! Config crate - 应用配置管理模块
//!
//! 此模块提供了集中的配置管理功能，包括：
//! - 加载和解析配置文件
//! - 支持环境变量覆盖配置
//! - 提供全局配置访问
//! - 配置结构体的定义和默认值

use std::sync::LazyLock;

mod app_settings_cfg;
mod database_cfg;
mod logging_cfg;
mod server_cfg;

use anyhow::{Context, anyhow};
use config::{Config, FileFormat};
use serde::Deserialize;
mod crypto;
use app_settings_cfg::AppSettings;
pub use crypto::*;
use database_cfg::DatabaseConfig;
use logging_cfg::LoggingConfig;
use server_cfg::ServerConfig;

/// 应用配置结构体
///
/// 包含服务器、数据库和应用设置的配置信息
#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
    #[serde(default)]
    app: AppSettings,
    #[serde(default)]
    logging: LoggingConfig,
}

impl AppConfig {
    /// 加载配置
    ///
    /// 从application.toml文件加载配置，并支持环境变量覆盖
    ///
    /// # Returns
    /// - 成功时返回AppConfig实例
    /// - 失败时返回anyhow::Error
    pub fn load() -> anyhow::Result<Self> {
        Config::builder()
            // 从application.toml文件加载配置
            .add_source(
                config::File::with_name("application").format(FileFormat::Toml).required(true),
            )
            // 从环境变量加载配置，前缀为APP
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

    /// 获取服务器配置
    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    /// 获取数据库配置
    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    /// 获取应用设置
    pub fn app(&self) -> &AppSettings {
        &self.app
    }

    /// 获取日志配置
    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }
}

// 全局配置实例
static CONFIG: LazyLock<AppConfig> =
    LazyLock::new(|| AppConfig::load().expect("Failed to initialize app_config"));

/// 获取全局配置
///
/// 返回一个静态引用到全局配置实例
///
/// # Panics
/// 如果配置初始化失败，会panic
pub fn get() -> &'static AppConfig {
    &CONFIG
}

/// 尝试获取全局配置
///
/// 如果配置尚未初始化，会尝试初始化
///
/// # Returns
/// - 成功时返回Ok(&'static AppConfig)
/// - 失败时返回Err(anyhow::Error)
pub fn try_get() -> anyhow::Result<&'static AppConfig> {
    // 尝试访问CONFIG，这会触发初始化
    Ok(&CONFIG)
}
