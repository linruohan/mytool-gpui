/// 统一错误处理系统
///
/// 提供统一的错误类型、错误处理和用户友好的错误提示
///
/// 功能：
/// - 统一的错误类型定义
/// - 错误分类和优先级
/// - 用户友好的错误消息
/// - 错误日志记录
/// - 错误恢复建议
use thiserror::Error;
use tracing::{error, info, warn};

// ==================== 错误类型定义 ====================

/// 应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(#[from] todos::error::TodoError),

    /// 验证错误
    #[error("验证错误: {0}")]
    Validation(String),

    /// 权限错误
    #[error("权限不足: {0}")]
    Permission(String),

    /// 资源未找到
    #[error("未找到: {0}")]
    NotFound(String),

    /// 网络错误
    #[error("网络错误: {0}")]
    Network(String),

    /// 文件系统错误
    #[error("文件系统错误: {0}")]
    FileSystem(#[from] std::io::Error),

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 解析错误
    #[error("解析错误: {0}")]
    Parse(String),

    /// 并发错误
    #[error("并发错误: {0}")]
    Concurrency(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    Internal(String),

    /// 用户取消操作
    #[error("操作已取消")]
    Cancelled,

    /// 超时错误
    #[error("操作超时: {0}")]
    Timeout(String),

    /// 其他错误
    #[error("错误: {0}")]
    Other(String),
}

// ==================== 错误严重程度 ====================

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 信息：不影响功能，仅供参考
    Info,
    /// 警告：可能影响功能，但可以继续
    Warning,
    /// 错误：影响功能，需要用户注意
    Error,
    /// 严重：严重影响功能，需要立即处理
    Critical,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "信息",
            Self::Warning => "警告",
            Self::Error => "错误",
            Self::Critical => "严重错误",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => "[INFO]",
            Self::Warning => "[WARN]",
            Self::Error => "[ERROR]",
            Self::Critical => "[CRITICAL]",
        }
    }
}

// ==================== 错误上下文 ====================

/// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 错误类型
    pub error: String,
    /// 错误严重程度
    pub severity: ErrorSeverity,
    /// 用户友好的错误消息
    pub user_message: String,
    /// 技术详情（用于日志）
    pub technical_details: String,
    /// 恢复建议
    pub recovery_suggestions: Vec<String>,
    /// 错误发生的位置
    pub location: Option<String>,
    /// 相关的资源 ID
    pub resource_id: Option<String>,
}

impl ErrorContext {
    pub fn new(error: AppError) -> Self {
        let (severity, user_message, recovery_suggestions) = match &error {
            AppError::Database(_e) => {
                (ErrorSeverity::Error, "数据库操作失败，请稍后重试".to_string(), vec![
                    "检查数据库文件是否存在".to_string(),
                    "尝试重启应用".to_string(),
                    "如果问题持续，请联系技术支持".to_string(),
                ])
            },
            AppError::Validation(msg) => {
                (ErrorSeverity::Warning, format!("输入验证失败: {}", msg), vec![
                    "请检查输入内容是否符合要求".to_string(),
                ])
            },
            AppError::Permission(msg) => {
                (ErrorSeverity::Error, format!("权限不足: {}", msg), vec![
                    "请检查您的权限设置".to_string(),
                    "联系管理员获取必要权限".to_string(),
                ])
            },
            AppError::NotFound(resource) => {
                (ErrorSeverity::Warning, format!("未找到: {}", resource), vec![
                    "请确认资源是否存在".to_string(),
                    "尝试刷新页面".to_string(),
                ])
            },
            AppError::Network(msg) => (ErrorSeverity::Error, format!("网络错误: {}", msg), vec![
                "检查网络连接".to_string(),
                "稍后重试".to_string(),
                "如果问题持续，请检查防火墙设置".to_string(),
            ]),
            AppError::FileSystem(e) => {
                (ErrorSeverity::Error, format!("文件操作失败: {}", e), vec![
                    "检查文件路径是否正确".to_string(),
                    "确认有足够的磁盘空间".to_string(),
                    "检查文件权限".to_string(),
                ])
            },
            AppError::Config(msg) => (ErrorSeverity::Critical, format!("配置错误: {}", msg), vec![
                "检查配置文件格式".to_string(),
                "恢复默认配置".to_string(),
                "重新安装应用".to_string(),
            ]),
            AppError::Parse(msg) => (ErrorSeverity::Warning, format!("解析失败: {}", msg), vec![
                "检查数据格式是否正确".to_string(),
            ]),
            AppError::Concurrency(msg) => {
                (ErrorSeverity::Warning, format!("并发冲突: {}", msg), vec![
                    "请稍后重试".to_string(),
                ])
            },
            AppError::Internal(_msg) => (
                ErrorSeverity::Critical,
                "应用内部错误，请联系技术支持".to_string(),
                vec!["尝试重启应用".to_string(), "如果问题持续，请报告此错误".to_string()],
            ),
            AppError::Cancelled => (ErrorSeverity::Info, "操作已取消".to_string(), vec![]),
            AppError::Timeout(msg) => (ErrorSeverity::Warning, format!("操作超时: {}", msg), vec![
                "请稍后重试".to_string(),
                "检查网络连接".to_string(),
            ]),
            AppError::Other(msg) => {
                (ErrorSeverity::Error, msg.clone(), vec!["请稍后重试".to_string()])
            },
        };

        Self {
            error: error.to_string(),
            severity,
            user_message,
            technical_details: format!("{:?}", error),
            recovery_suggestions,
            location: None,
            resource_id: None,
        }
    }

    /// 设置错误发生的位置
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// 设置相关的资源 ID
    pub fn with_resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    /// 记录错误日志
    pub fn log(&self) {
        let location_str = self.location.as_deref().unwrap_or("unknown");
        let resource_str = self.resource_id.as_deref().unwrap_or("none");

        match self.severity {
            ErrorSeverity::Info => {
                info!(
                    location = location_str,
                    resource_id = resource_str,
                    "{}",
                    self.technical_details
                );
            },
            ErrorSeverity::Warning => {
                warn!(
                    location = location_str,
                    resource_id = resource_str,
                    "{}",
                    self.technical_details
                );
            },
            ErrorSeverity::Error | ErrorSeverity::Critical => {
                error!(
                    location = location_str,
                    resource_id = resource_str,
                    severity = ?self.severity,
                    "{}",
                    self.technical_details
                );
            },
        }
    }

    /// 生成用户友好的错误消息
    pub fn format_user_message(&self) -> String {
        let mut message = format!("{} {}\n\n", self.severity.icon(), self.user_message);

        if !self.recovery_suggestions.is_empty() {
            message.push_str("建议：\n");
            for (i, suggestion) in self.recovery_suggestions.iter().enumerate() {
                message.push_str(&format!("{}. {}\n", i + 1, suggestion));
            }
        }

        message
    }
}

// ==================== 错误处理器 ====================

/// 错误处理器
pub struct ErrorHandler;

impl ErrorHandler {
    /// 处理错误
    pub fn handle(error: AppError) -> ErrorContext {
        let context = ErrorContext::new(error);
        context.log();
        context
    }

    /// 处理带位置的错误
    pub fn handle_with_location(error: AppError, location: impl Into<String>) -> ErrorContext {
        let context = ErrorContext::new(error).with_location(location);
        context.log();
        context
    }

    /// 处理带资源 ID 的错误
    pub fn handle_with_resource(
        error: AppError,
        location: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> ErrorContext {
        let context =
            ErrorContext::new(error).with_location(location).with_resource_id(resource_id);
        context.log();
        context
    }
}

// ==================== 结果类型别名 ====================

/// 应用结果类型
pub type AppResult<T> = Result<T, AppError>;

// ==================== 错误转换 ====================

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.to_string())
    }
}

// ==================== 验证辅助函数 ====================

/// 验证辅助函数
pub mod validation {
    use super::*;

    /// 验证任务内容
    pub fn validate_task_content(content: &str) -> AppResult<()> {
        if content.trim().is_empty() {
            return Err(AppError::Validation("任务内容不能为空".to_string()));
        }

        if content.len() > 10000 {
            return Err(AppError::Validation("任务内容过长（最多 10000 字符）".to_string()));
        }

        // 检查危险字符
        if content.contains("<script>") || content.contains("javascript:") {
            return Err(AppError::Validation("任务内容包含不安全的字符".to_string()));
        }

        Ok(())
    }

    /// 验证项目名称
    pub fn validate_project_name(name: &str) -> AppResult<()> {
        if name.trim().is_empty() {
            return Err(AppError::Validation("项目名称不能为空".to_string()));
        }

        if name.len() > 200 {
            return Err(AppError::Validation("项目名称过长（最多 200 字符）".to_string()));
        }

        Ok(())
    }

    /// 验证标签名称
    pub fn validate_label_name(name: &str) -> AppResult<()> {
        if name.trim().is_empty() {
            return Err(AppError::Validation("标签名称不能为空".to_string()));
        }

        if name.len() > 50 {
            return Err(AppError::Validation("标签名称过长（最多 50 字符）".to_string()));
        }

        Ok(())
    }

    /// 清理 HTML 内容
    pub fn sanitize_html(content: &str) -> String {
        content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let error = AppError::Validation("测试错误".to_string());
        let context = ErrorContext::new(error);

        assert_eq!(context.severity, ErrorSeverity::Warning);
        assert!(context.user_message.contains("验证失败"));
        assert!(!context.recovery_suggestions.is_empty());
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
    }

    #[test]
    fn test_validation_task_content() {
        // 空内容
        assert!(validation::validate_task_content("").is_err());
        assert!(validation::validate_task_content("   ").is_err());

        // 正常内容
        assert!(validation::validate_task_content("正常任务").is_ok());

        // 过长内容
        let long_content = "a".repeat(10001);
        assert!(validation::validate_task_content(&long_content).is_err());

        // 危险内容
        assert!(validation::validate_task_content("<script>alert('xss')</script>").is_err());
    }

    #[test]
    fn test_sanitize_html() {
        let input = "<script>alert('test')</script>";
        let output = validation::sanitize_html(input);
        assert!(!output.contains("<script>"));
        assert!(output.contains("&lt;script&gt;"));
    }
}
