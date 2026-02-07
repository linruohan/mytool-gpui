//! Config crate - 应用配置管理模块
//!
//! 此模块提供了集中的配置管理功能，包括：
//! - 加载和解析配置文件
//! - 支持环境变量覆盖配置
//! - 提供全局配置访问
//! - 配置结构体的定义和默认值
//! - 支持多环境配置（dev/prod/test）
//! - 配置热重载

use std::{
    path::{Path, PathBuf},
    sync::{LazyLock, RwLock},
};

mod app_settings_cfg;
mod database_cfg;
mod logging_cfg;
mod server_cfg;

use anyhow::{Context, Result, anyhow, bail};
use config::{Config, FileFormat};
use serde::Deserialize;
mod crypto;
use app_settings_cfg::AppSettings;
pub use crypto::*;
use database_cfg::DatabaseConfig;
use logging_cfg::LoggingConfig;
use server_cfg::ServerConfig;

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
    Test,
}

impl Environment {
    /// 从环境变量获取当前环境
    pub fn current() -> Self {
        std::env::var("APP_ENV")
            .or_else(|_| std::env::var("RUST_ENV"))
            .ok()
            .and_then(|env| match env.to_lowercase().as_str() {
                "prod" | "production" => Some(Self::Production),
                "test" | "testing" => Some(Self::Test),
                "dev" | "development" => Some(Self::Development),
                _ => None,
            })
            .unwrap_or(Self::Development)
    }

    /// 获取环境名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
            Self::Test => "test",
        }
    }
}

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
    #[serde(skip)]
    config_path: Option<PathBuf>,
}

impl AppConfig {
    /// 查找配置文件路径
    ///
    /// 按优先级查找配置文件：
    /// 1. 环境特定配置 (application.{env}.toml)
    /// 2. 默认配置 (application.toml)
    fn find_config_file(env: Environment) -> Result<PathBuf> {
        let search_paths = [".", "..", "../.."];
        let env_filename = format!("application.{}.toml", env.as_str());
        let default_filename = "application.toml";

        // 优先查找环境特定配置
        for base_path in &search_paths {
            let env_path = Path::new(base_path).join(&env_filename);
            if env_path.exists() {
                return Ok(env_path);
            }
        }

        // 查找默认配置
        for base_path in &search_paths {
            let default_path = Path::new(base_path).join(default_filename);
            if default_path.exists() {
                return Ok(default_path);
            }
        }

        bail!(
            "配置文件未找到。尝试查找: {} 或 {} 在路径: {:?}",
            env_filename,
            default_filename,
            search_paths
        )
    }

    /// 加载配置
    ///
    /// 从application.toml文件加载配置，并支持环境变量覆盖
    ///
    /// # Returns
    /// - 成功时返回AppConfig实例
    /// - 失败时返回anyhow::Error
    pub fn load() -> Result<Self> {
        Self::load_with_env(Environment::current())
    }

    /// 使用指定环境加载配置
    pub fn load_with_env(env: Environment) -> Result<Self> {
        let config_path = Self::find_config_file(env).with_context(|| "查找配置文件失败")?;

        let mut builder = Config::builder();

        // 加载主配置文件
        builder = builder.add_source(
            config::File::from(config_path.as_path()).format(FileFormat::Toml).required(true),
        );

        // 从环境变量加载配置，前缀为APP
        builder = builder.add_source(
            config::Environment::with_prefix("APP")
                .try_parsing(true)
                .separator("_")
                .list_separator(","),
        );

        let mut config: Self = builder
            .build()
            .with_context(|| format!("构建配置失败，文件: {:?}", config_path))?
            .try_deserialize()
            .with_context(|| format!("反序列化配置失败，文件: {:?}", config_path))?;

        config.config_path = Some(config_path);
        config.validate()?;

        Ok(config)
    }

    /// 验证配置有效性
    fn validate(&self) -> Result<()> {
        // 验证端口范围
        let port = self.server.port();
        if port == 0 {
            bail!("服务器端口不能为0");
        }

        // 验证数据库连接池大小
        let pool_size = self.database.pool_size();
        if pool_size == 0 {
            bail!("数据库连接池大小必须大于0");
        }
        if pool_size > 1000 {
            bail!("数据库连接池大小过大: {}", pool_size);
        }

        // 验证日志级别
        let level = self.logging.level();
        if !["trace", "debug", "info", "warn", "error"].contains(&level) {
            bail!("无效的日志级别: {}", level);
        }

        Ok(())
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
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

// 全局配置实例（支持重载）
static CONFIG: LazyLock<RwLock<AppConfig>> =
    LazyLock::new(|| RwLock::new(AppConfig::load().expect("初始化配置失败")));

/// 获取全局配置
///
/// 返回一个静态引用到全局配置实例
///
/// # Panics
/// 如果配置初始化失败，会panic
pub fn get() -> &'static RwLock<AppConfig> {
    &CONFIG
}

/// 尝试获取全局配置
///
/// 如果配置尚未初始化，会尝试初始化
///
/// # Returns
/// - 成功时返回Ok(&'static RwLock<AppConfig>)
/// - 失败时返回Err(anyhow::Error)
pub fn try_get() -> Result<&'static RwLock<AppConfig>> {
    Ok(&CONFIG)
}

/// 重载全局配置
///
/// 重新从配置文件加载配置并更新全局实例
///
/// # Returns
/// - 成功时返回Ok(())
/// - 失败时返回Err(anyhow::Error)
pub fn reload() -> Result<()> {
    let new_config = AppConfig::load().with_context(|| "重载配置失败")?;

    let mut config = CONFIG.write().map_err(|e| anyhow!("获取配置写锁失败: {}", e))?;

    *config = new_config;
    Ok(())
}

/// 使用指定环境重载配置
pub fn reload_with_env(env: Environment) -> Result<()> {
    let new_config = AppConfig::load_with_env(env)
        .with_context(|| format!("使用环境 {} 重载配置失败", env.as_str()))?;

    let mut config = CONFIG.write().map_err(|e| anyhow!("获取配置写锁失败: {}", e))?;

    *config = new_config;
    Ok(())
}

/// 便捷宏：访问配置
///
/// # Examples
/// ```ignore
/// use gconfig::config;
///
/// let port = config!(|cfg| cfg.server().port());
/// let db_host = config!(|cfg| cfg.database().host().to_string());
/// ```
#[macro_export]
macro_rules! config {
    (|$cfg:ident| $expr:expr) => {{
        let guard = $crate::get().read().expect("读取配置失败");
        let $cfg = &*guard;
        $expr
    }};
}
