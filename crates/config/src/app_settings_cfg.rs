use serde::Deserialize;

/// 应用设置结构体
#[derive(Deserialize, Debug, Clone, Default)]
pub struct AppSettings {
    #[serde(default = "default_language")]
    language: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_clock_format")]
    clock_format: String,
}

/// 默认语言
fn default_language() -> String {
    "en".to_string()
}

/// 默认主题
fn default_theme() -> String {
    "light".to_string()
}

/// 默认时钟格式
fn default_clock_format() -> String {
    "24h".to_string()
}

impl AppSettings {
    /// 获取语言设置
    pub fn language(&self) -> &str {
        &self.language
    }

    /// 获取主题设置
    pub fn theme(&self) -> &str {
        &self.theme
    }

    /// 获取时钟格式
    pub fn clock_format(&self) -> &str {
        &self.clock_format
    }

    /// 检查是否使用24小时制
    pub fn is_24h_clock(&self) -> bool {
        self.clock_format == "24h"
    }
}
