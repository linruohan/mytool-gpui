//! 日志配置模块

use serde::Deserialize;

/// 日志配置结构体
#[derive(Deserialize, Debug, Clone)]
pub struct LoggingConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self { level: default_log_level() }
    }
}

impl LoggingConfig {
    /// 获取日志级别
    pub fn level(&self) -> &str {
        &self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level(), "info");
    }
}
