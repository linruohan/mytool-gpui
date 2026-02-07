use serde::Deserialize;

/// 日志配置结构体
#[derive(Deserialize, Debug, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default = "default_log_file")]
    file: Option<String>,
}

/// 默认日志级别
fn default_log_level() -> String {
    "info".to_string()
}

/// 默认日志文件
fn default_log_file() -> Option<String> {
    None
}

impl LoggingConfig {
    /// 获取日志级别
    pub fn level(&self) -> &str {
        &self.level
    }

    /// 获取日志文件路径
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}
