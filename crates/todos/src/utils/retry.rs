//! 通用重试操作工具模块
//!
//! 提供可配置的重试机制，用于处理临时性错误（如数据库锁定、网络超时等）。
//!
//! ## 特性
//! - 可配置的最大重试次数和延迟
//! - 指数退避策略
//! - 支持异步操作
//! - 与 `TodoError` 集成，自动判断可重试错误

use std::time::Duration;

use tokio::time::sleep;

use crate::error::TodoError;

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: usize,
    /// 初始延迟（毫秒）
    pub initial_delay_ms: u64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
    /// 是否使用指数退避
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            exponential_backoff: true,
        }
    }
}

impl RetryConfig {
    /// 创建新的重试配置
    pub fn new(max_retries: usize, initial_delay_ms: u64) -> Self {
        Self { max_retries, initial_delay_ms, max_delay_ms: 5000, exponential_backoff: true }
    }

    /// 计算指定重试次数的延迟时间
    fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let delay_ms = if self.exponential_backoff {
            let delay = self.initial_delay_ms * (2_u64.pow(attempt as u32));
            delay.min(self.max_delay_ms)
        } else {
            self.initial_delay_ms
        };
        Duration::from_millis(delay_ms)
    }
}

/// 重试操作结果
#[derive(Debug)]
pub struct RetryResult<T> {
    /// 操作结果
    pub result: T,
    /// 总重试次数（0 表示首次成功）
    pub retry_count: usize,
}

impl<T> RetryResult<T> {
    /// 创建新的重试结果
    pub fn new(result: T, retry_count: usize) -> Self {
        Self { result, retry_count }
    }

    /// 检查是否进行过重试
    pub fn had_retries(&self) -> bool {
        self.retry_count > 0
    }
}

/// 使用默认配置执行可重试的异步操作
///
/// # 参数
/// - `operation_name`: 操作名称（用于日志）
/// - `operation`: 异步操作闭包
///
/// # 返回
/// - `Ok(T)`: 操作成功
/// - `Err(TodoError)`: 所有重试都失败
///
/// # 示例
/// ```ignore
/// let result = retry_operation("get_item", || async {
///     item_repo.find_by_id(id).await
/// }).await?;
/// ```
pub async fn retry_operation<F, Fut, T>(operation_name: &str, operation: F) -> Result<T, TodoError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TodoError>>,
{
    retry_operation_with_config(operation_name, operation, RetryConfig::default()).await
}

/// 使用自定义配置执行可重试的异步操作
///
/// # 参数
/// - `operation_name`: 操作名称（用于日志）
/// - `operation`: 异步操作闭包
/// - `config`: 重试配置
///
/// # 返回
/// - `Ok(RetryResult<T>)`: 操作成功，包含重试次数
/// - `Err(TodoError)`: 所有重试都失败
pub async fn retry_operation_with_config<F, Fut, T>(
    operation_name: &str,
    operation: F,
    config: RetryConfig,
) -> Result<T, TodoError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TodoError>>,
{
    let mut last_error: Option<TodoError> = None;

    for attempt in 0..config.max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    tracing::info!("✅ 重试成功: {} (第 {} 次尝试)", operation_name, attempt + 1);
                }
                return Ok(result);
            },
            Err(e) => {
                if e.is_retryable() {
                    if attempt < config.max_retries - 1 {
                        let delay = config.delay_for_attempt(attempt);
                        tracing::warn!(
                            "⚠️ 操作失败，准备重试: {} (第 {}/{} 次) - 错误: {:?}",
                            operation_name,
                            attempt + 1,
                            config.max_retries,
                            e
                        );
                        sleep(delay).await;
                    } else {
                        tracing::error!(
                            "❌ 所有重试失败: {} (共 {} 次) - 错误: {:?}",
                            operation_name,
                            config.max_retries,
                            e
                        );
                    }
                    last_error = Some(e);
                } else {
                    tracing::error!("❌ 不可重试的错误: {} - 错误: {:?}", operation_name, e);
                    return Err(e);
                }
            },
        }
    }

    Err(last_error
        .unwrap_or_else(|| TodoError::InternalError(format!("未知错误: {}", operation_name))))
}

/// 带上下文的重试操作
///
/// 在错误中添加操作上下文信息
pub async fn retry_with_context<F, Fut, T>(
    operation_name: &str,
    entity_type: &str,
    entity_id: &str,
    operation: F,
) -> Result<T, TodoError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TodoError>>,
{
    retry_operation(operation_name, operation)
        .await
        .map_err(|e| e.with_entity(entity_type, entity_id).with_operation(operation_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert!(config.exponential_backoff);
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_retry_success_first_try() {
        let result: Result<i32, TodoError> = retry_operation("test", || async { Ok(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let result: Result<i32, TodoError> = retry_operation("test", || async {
            Err(TodoError::ValidationError("不可重试".to_string()))
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TodoError::ValidationError(_)));
    }
}
