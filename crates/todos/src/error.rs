//! 错误处理模块
//!
//! 提供统一的错误类型和错误处理机制，支持：
//! - 错误分类和错误码
//! - 错误上下文信息
//! - 错误链追踪
//! - 用户友好的错误消息

use std::fmt;

use thiserror::Error;

/// 错误码枚举
///
/// 每种错误都有唯一的错误码，便于日志追踪和错误定位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// 数据库错误 (1xxx)
    DatabaseError = 1000,
    /// 实体未找到 (2xxx)
    NotFound = 2000,
    /// ID 未找到 (2xxx)
    IDNotFound = 2001,
    /// 实体已存在 (3xxx)
    AlreadyExists = 3000,
    /// 验证错误 (4xxx)
    ValidationError = 4000,
    /// 操作超时 (5xxx)
    Timeout = 5000,
    /// 权限错误 (6xxx)
    PermissionDenied = 6000,
    /// 配置错误 (7xxx)
    ConfigError = 7000,
    /// 内部错误 (9xxx)
    InternalError = 9000,
}

impl ErrorCode {
    /// 获取错误码数值
    pub fn code(&self) -> u32 {
        *self as u32
    }

    /// 获取错误码前缀（用于分类）
    pub fn category(&self) -> &'static str {
        match self {
            Self::DatabaseError => "DATABASE",
            Self::NotFound | Self::IDNotFound => "NOT_FOUND",
            Self::AlreadyExists => "CONFLICT",
            Self::ValidationError => "VALIDATION",
            Self::Timeout => "TIMEOUT",
            Self::PermissionDenied => "PERMISSION",
            Self::ConfigError => "CONFIG",
            Self::InternalError => "INTERNAL",
        }
    }

    /// 判断是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout | Self::DatabaseError)
    }

    /// 判断是否为客户端错误
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::NotFound | Self::IDNotFound | Self::AlreadyExists | Self::ValidationError
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.category(), self.code())
    }
}

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorSeverity {
    /// 警告 - 可恢复的小问题
    Warning,
    /// 错误 - 需要处理的错误
    #[default]
    Error,
    /// 严重 - 系统级错误
    Critical,
}

/// 错误上下文信息
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// 操作名称
    pub operation: Option<String>,
    /// 实体类型
    pub entity_type: Option<String>,
    /// 实体 ID
    pub entity_id: Option<String>,
    /// 附加信息
    pub details: Option<String>,
    /// 来源位置（文件:行号）
    pub location: Option<String>,
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置操作名称
    pub fn operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    /// 设置实体类型
    pub fn entity_type(mut self, t: impl Into<String>) -> Self {
        self.entity_type = Some(t.into());
        self
    }

    /// 设置实体 ID
    pub fn entity_id(mut self, id: impl Into<String>) -> Self {
        self.entity_id = Some(id.into());
        self
    }

    /// 设置附加信息
    pub fn details(mut self, d: impl Into<String>) -> Self {
        self.details = Some(d.into());
        self
    }

    /// 设置来源位置
    pub fn location(mut self, loc: impl Into<String>) -> Self {
        self.location = Some(loc.into());
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let Some(ref op) = self.operation {
            parts.push(format!("operation={}", op));
        }
        if let Some(ref t) = self.entity_type {
            parts.push(format!("entity_type={}", t));
        }
        if let Some(ref id) = self.entity_id {
            parts.push(format!("entity_id={}", id));
        }
        if let Some(ref d) = self.details {
            parts.push(format!("details={}", d));
        }
        if let Some(ref loc) = self.location {
            parts.push(format!("location={}", loc));
        }

        write!(f, "[{}]", parts.join(", "))
    }
}

/// 统一错误类型
#[derive(Error, Debug)]
pub enum TodoError {
    /// 数据库错误（来自 SeaORM）
    #[error("Database error: {0}")]
    DbError(#[from] sea_orm::DbErr),

    /// 数据库错误（自定义消息）
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// 实体未找到
    #[error("{0} not found")]
    NotFound(String),

    /// ID 未找到
    #[error("ID not found")]
    IDNotFound,

    /// 实体已存在
    #[error("{0} already exists")]
    AlreadyExists(String),

    /// 操作超时
    #[error("Operation timeout: {0}")]
    Timeout(String),

    /// 验证错误
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// 权限拒绝
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// 配置错误
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// 带上下文的错误
    #[error("{message} {context}")]
    WithContext { message: String, context: ErrorContext, source: Box<TodoError> },

    /// 内部错误
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl TodoError {
    /// 获取错误码
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::DbError(_) | Self::DatabaseError(_) => ErrorCode::DatabaseError,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::IDNotFound => ErrorCode::IDNotFound,
            Self::AlreadyExists(_) => ErrorCode::AlreadyExists,
            Self::Timeout(_) => ErrorCode::Timeout,
            Self::ValidationError(_) => ErrorCode::ValidationError,
            Self::PermissionDenied(_) => ErrorCode::PermissionDenied,
            Self::ConfigError(_) => ErrorCode::ConfigError,
            Self::WithContext { source, .. } => source.error_code(),
            Self::InternalError(_) => ErrorCode::InternalError,
        }
    }

    /// 获取错误严重程度
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::DbError(_) | Self::DatabaseError(_) => ErrorSeverity::Error,
            Self::NotFound(_) | Self::IDNotFound => ErrorSeverity::Warning,
            Self::AlreadyExists(_) => ErrorSeverity::Warning,
            Self::Timeout(_) => ErrorSeverity::Error,
            Self::ValidationError(_) => ErrorSeverity::Warning,
            Self::PermissionDenied(_) => ErrorSeverity::Error,
            Self::ConfigError(_) => ErrorSeverity::Critical,
            Self::WithContext { source, .. } => source.severity(),
            Self::InternalError(_) => ErrorSeverity::Critical,
        }
    }

    /// 判断是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        self.error_code().is_retryable()
    }

    /// 判断是否为客户端错误
    pub fn is_client_error(&self) -> bool {
        self.error_code().is_client_error()
    }

    /// 添加上下文信息
    pub fn with_context(self, context: ErrorContext) -> Self {
        let message = self.to_string();
        Self::WithContext { message, context, source: Box::new(self) }
    }

    /// 快速添加操作上下文
    pub fn with_operation(self, operation: impl Into<String>) -> Self {
        self.with_context(ErrorContext::new().operation(operation))
    }

    /// 快速添加实体上下文
    pub fn with_entity(self, entity_type: impl Into<String>, entity_id: impl Into<String>) -> Self {
        self.with_context(ErrorContext::new().entity_type(entity_type).entity_id(entity_id))
    }

    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        match self {
            Self::DbError(_) | Self::DatabaseError(_) => "数据库操作失败，请稍后重试".to_string(),
            Self::NotFound(entity) => format!("找不到: {}", entity),
            Self::IDNotFound => "找不到指定的 ID".to_string(),
            Self::AlreadyExists(entity) => format!("已存在: {}", entity),
            Self::Timeout(operation) => format!("操作超时: {}", operation),
            Self::ValidationError(msg) => format!("数据验证失败: {}", msg),
            Self::PermissionDenied(msg) => format!("权限不足: {}", msg),
            Self::ConfigError(msg) => format!("配置错误: {}", msg),
            Self::WithContext { message, .. } => message.clone(),
            Self::InternalError(msg) => format!("内部错误: {}", msg),
        }
    }

    /// 获取错误链
    pub fn chain(&self) -> Vec<&TodoError> {
        let mut chain = vec![self];
        if let Self::WithContext { source, .. } = self {
            chain.extend(source.chain());
        }
        chain
    }

    /// 创建验证错误
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }

    /// 创建未找到错误
    pub fn not_found(entity: impl Into<String>) -> Self {
        Self::NotFound(entity.into())
    }

    /// 创建已存在错误
    pub fn already_exists(entity: impl Into<String>) -> Self {
        Self::AlreadyExists(entity.into())
    }

    /// 创建超时错误
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout(operation.into())
    }
}

/// 结果类型别名
pub type TodoResult<T> = Result<T, TodoError>;

/// 错误处理辅助宏
#[macro_export]
macro_rules! bail {
    ($err:expr) => {
        return Err($err.into())
    };
    ($err:expr, $($arg:tt)*) => {
        return Err($crate::error::TodoError::from($err).with_operation(format!($($arg)*)))
    };
}

/// 创建带位置的错误上下文
#[macro_export]
macro_rules! error_context {
    ($operation:expr) => {
        $crate::error::ErrorContext::new().operation($operation).location(concat!(
            file!(),
            ":",
            line!()
        ))
    };
    ($operation:expr, $entity_type:expr, $entity_id:expr) => {
        $crate::error::ErrorContext::new()
            .operation($operation)
            .entity_type($entity_type)
            .entity_id($entity_id)
            .location(concat!(file!(), ":", line!()))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        assert_eq!(ErrorCode::DatabaseError.code(), 1000);
        assert_eq!(ErrorCode::NotFound.code(), 2000);
        assert_eq!(ErrorCode::ValidationError.code(), 4000);
        assert!(ErrorCode::Timeout.is_retryable());
        assert!(ErrorCode::NotFound.is_client_error());
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new().operation("create_item").entity_type("Item").entity_id("123");

        assert_eq!(ctx.operation, Some("create_item".to_string()));
        assert_eq!(ctx.entity_type, Some("Item".to_string()));
        assert_eq!(ctx.entity_id, Some("123".to_string()));
    }

    #[test]
    fn test_todo_error() {
        let err = TodoError::not_found("Item 123");
        assert_eq!(err.error_code(), ErrorCode::NotFound);
        assert_eq!(err.severity(), ErrorSeverity::Warning);
        assert!(err.is_client_error());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_with_context() {
        let err = TodoError::not_found("Item")
            .with_context(ErrorContext::new().operation("get_item").entity_id("123"));

        assert_eq!(err.error_code(), ErrorCode::NotFound);
        let chain = err.chain();
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_user_message() {
        let err = TodoError::not_found("项目");
        assert_eq!(err.user_message(), "找不到: 项目");

        let err = TodoError::validation("名称不能为空");
        assert_eq!(err.user_message(), "数据验证失败: 名称不能为空");
    }

    #[test]
    fn test_convenience_methods() {
        let err = TodoError::validation("test");
        assert!(matches!(err, TodoError::ValidationError(_)));

        let err = TodoError::not_found("test");
        assert!(matches!(err, TodoError::NotFound(_)));

        let err = TodoError::already_exists("test");
        assert!(matches!(err, TodoError::AlreadyExists(_)));

        let err = TodoError::timeout("test");
        assert!(matches!(err, TodoError::Timeout(_)));
    }
}
