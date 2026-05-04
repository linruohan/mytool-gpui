//! 日志配置模块
//!
//! 提供结构化日志、追踪 ID、性能指标等功能。

use serde::Deserialize;

/// 日志格式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// 纯文本格式（默认）
    #[default]
    Text,
    /// JSON 结构化格式
    Json,
    /// 紧凑格式
    Compact,
}

/// 日志输出目标
#[derive(Debug, Clone, Deserialize)]
pub struct LogOutput {
    /// 是否输出到控制台
    #[serde(default = "default_console")]
    pub console: bool,
    /// 日志文件路径（可选）
    pub file: Option<String>,
    /// 文件最大大小（MB），默认 10MB
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    /// 保留的文件数量，默认 5
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_console() -> bool {
    true
}

fn default_max_file_size() -> u64 {
    10
}

fn default_max_files() -> usize {
    5
}

impl Default for LogOutput {
    fn default() -> Self {
        Self { console: true, file: None, max_file_size: 10, max_files: 5 }
    }
}

/// 追踪配置
#[derive(Debug, Clone, Deserialize)]
pub struct TracingConfig {
    /// 是否启用请求追踪 ID
    #[serde(default)]
    pub enabled: bool,
    /// 追踪 ID 头名称
    #[serde(default = "default_trace_header")]
    pub header_name: String,
}

fn default_trace_header() -> String {
    "X-Request-Id".to_string()
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self { enabled: false, header_name: default_trace_header() }
    }
}

/// 性能指标配置
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    /// 是否启用性能指标采集
    #[serde(default)]
    pub enabled: bool,
    /// 慢操作阈值（毫秒），超过此阈值记录警告
    #[serde(default = "default_slow_threshold")]
    pub slow_threshold_ms: u64,
    /// 是否记录内存使用
    #[serde(default)]
    pub record_memory: bool,
}

fn default_slow_threshold() -> u64 {
    1000
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { enabled: false, slow_threshold_ms: 1000, record_memory: false }
    }
}

/// 日志配置结构体
#[derive(Deserialize, Debug, Clone)]
pub struct LoggingConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    level: String,
    /// 日志格式
    #[serde(default)]
    pub format: LogFormat,
    /// 输出配置
    #[serde(default)]
    pub output: LogOutput,
    /// 追踪配置
    #[serde(default)]
    pub tracing: TracingConfig,
    /// 性能指标配置
    #[serde(default)]
    pub metrics: MetricsConfig,
    /// 是否显示文件和行号
    #[serde(default = "default_show_location")]
    pub show_location: bool,
    /// 是否显示线程 ID
    #[serde(default)]
    pub show_thread_id: bool,
    /// 是否显示目标模块
    #[serde(default = "default_show_target")]
    pub show_target: bool,
}

/// 默认日志级别
fn default_log_level() -> String {
    "info".to_string()
}

fn default_show_location() -> bool {
    true
}

fn default_show_target() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: LogFormat::default(),
            output: LogOutput::default(),
            tracing: TracingConfig::default(),
            metrics: MetricsConfig::default(),
            show_location: true,
            show_thread_id: false,
            show_target: true,
        }
    }
}

impl LoggingConfig {
    /// 获取日志级别
    pub fn level(&self) -> &str {
        &self.level
    }

    /// 获取日志文件路径
    pub fn file(&self) -> Option<&str> {
        self.output.file.as_deref()
    }

    /// 是否启用 JSON 格式
    pub fn is_json_format(&self) -> bool {
        self.format == LogFormat::Json
    }

    /// 是否启用追踪 ID
    pub fn is_tracing_enabled(&self) -> bool {
        self.tracing.enabled
    }

    /// 是否启用性能指标
    pub fn is_metrics_enabled(&self) -> bool {
        self.metrics.enabled
    }

    /// 获取慢操作阈值（毫秒）
    pub fn slow_threshold_ms(&self) -> u64 {
        self.metrics.slow_threshold_ms
    }

    /// 解析日志级别字符串为 tracing 级别
    pub fn parse_level(&self) -> tracing::Level {
        match self.level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" | "warning" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    }
}

/// 生成追踪 ID
#[allow(dead_code)]
pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 性能计时器
#[allow(dead_code)]
pub struct PerformanceTimer {
    name: String,
    start: std::time::Instant,
    threshold_ms: u64,
}

#[allow(dead_code)]
impl PerformanceTimer {
    /// 创建新的性能计时器
    pub fn new(name: impl Into<String>, threshold_ms: u64) -> Self {
        Self { name: name.into(), start: std::time::Instant::now(), threshold_ms }
    }

    /// 结束计时并记录
    pub fn finish(&self) -> u64 {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        if elapsed_ms > self.threshold_ms {
            tracing::warn!(
                operation = %self.name,
                elapsed_ms = elapsed_ms,
                threshold_ms = self.threshold_ms,
                "慢操作检测"
            );
        } else {
            tracing::debug!(
                operation = %self.name,
                elapsed_ms = elapsed_ms,
                "操作完成"
            );
        }
        elapsed_ms
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        self.finish();
    }
}

/// 结构化日志字段辅助宏
#[macro_export]
macro_rules! log_fields {
    ($($key:ident = $value:expr),* $(,)?) => {
        $($crate::log_field!($key, $value);)*
    };
}

/// 记录操作日志的辅助函数
#[allow(dead_code)]
pub fn log_operation(
    operation: &str,
    entity_type: &str,
    entity_id: &str,
    success: bool,
    elapsed_ms: Option<u64>,
) {
    if success {
        tracing::info!(
            operation = operation,
            entity_type = entity_type,
            entity_id = entity_id,
            elapsed_ms = ?elapsed_ms,
            "操作成功"
        );
    } else {
        tracing::warn!(
            operation = operation,
            entity_type = entity_type,
            entity_id = entity_id,
            elapsed_ms = ?elapsed_ms,
            "操作失败"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level(), "info");
        assert_eq!(config.format, LogFormat::Text);
        assert!(config.output.console);
        assert!(!config.tracing.enabled);
        assert!(!config.metrics.enabled);
    }

    #[test]
    fn test_parse_level() {
        let config = LoggingConfig { level: "debug".to_string(), ..Default::default() };
        assert_eq!(config.parse_level(), tracing::Level::DEBUG);

        let config = LoggingConfig { level: "warn".to_string(), ..Default::default() };
        assert_eq!(config.parse_level(), tracing::Level::WARN);
    }

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36);
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_operation", 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.finish();
        assert!(elapsed >= 10);
    }
}
