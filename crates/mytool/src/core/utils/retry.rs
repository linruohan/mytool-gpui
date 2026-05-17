//! 重试机制 - 为异步操作提供自动重试能力
//!
//! 使用指数退避策略，避免在网络抖动时频繁重试导致雪崩。
//!
//! ## 设计原则
//! - 只对临时性错误重试（网络超时、连接中断等）
//! - 不对逻辑错误重试（数据验证失败、权限不足等）
//! - 指数退避 + 抖动，避免惊群效应

use std::time::Duration;

use todos::error::TodoError;
use tracing::{info, warn};

/// 判断 TodoError 是否可重试
///
/// 数据库错误和超时错误通常是可以恢复的（如网络抖动、锁竞争），
/// 而验证错误、未找到等逻辑错误不应重试。
impl crate::core::utils::retry::Retryable for TodoError {
    fn should_retry(&self) -> bool {
        matches!(self, TodoError::DbError(_) | TodoError::DatabaseError(_) | TodoError::Timeout(_))
    }
}

/// 重试 trait（本地定义，避免跨 crate 实现问题）
pub trait Retryable {
    fn should_retry(&self) -> bool;
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数（不包括首次尝试）
    pub max_retries: u32,
    /// 初始重试间隔
    pub initial_delay: Duration,
    /// 退避倍数（每次重试的间隔 = 上次间隔 * multiplier）
    pub multiplier: f64,
    /// 最大延迟上限
    pub max_delay: Duration,
    /// 是否启用随机抖动（避免多个客户端同时重试）
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            multiplier: 2.0,
            max_delay: Duration::from_secs(5),
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// 创建数据库操作专用的重试配置
    ///
    /// 数据库操作通常需要较长的超时时间和较少的重试次数
    pub fn for_db_operation() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Duration::from_secs(8),
            jitter: true,
        }
    }

    /// 计算第 `attempt` 次重试的等待时间
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let mut delay = self.initial_delay;

        // 应用指数退避
        for _ in 0..attempt {
            let delay_ms = (delay.as_millis() as f64 * self.multiplier)
                .min(self.max_delay.as_millis() as f64) as u64;
            delay = Duration::from_millis(delay_ms);
        }

        // 应用随机抖动（±25%）
        if self.jitter {
            let jitter_range = delay.as_millis() as f64 * 0.25;
            // 使用简单的伪随机数生成（基于时间戳低位）
            let pseudo_random = ((std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64)
                & 0xFF) as f64
                / 255.0;
            let jitter_value = pseudo_random * jitter_range;
            let final_delay = delay.as_millis() as f64 + (jitter_value - jitter_range / 2.0);
            delay = Duration::from_millis(final_delay.max(0.0) as u64);
        }

        delay
    }
}

/// 带重试的异步操作执行器（专门用于 TodoError）
///
/// # 示例
/// ```ignore
/// let result = retry_async_todo(
///     || async { db.save_item(item).await },
///     RetryConfig::for_db_operation(),
/// ).await;
/// ```
pub async fn retry_async_todo<F, Fut, T>(operation: F, config: RetryConfig) -> Result<T, TodoError>
where
    F: Fn(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, TodoError>>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match operation(attempt).await {
            Ok(result) => {
                if attempt > 0 {
                    info!("Retry succeeded on attempt {}/{}", attempt, config.max_retries);
                }
                return Ok(result);
            },
            Err(e) => {
                warn!("Operation failed on attempt {}/{}: {:?}", attempt, config.max_retries, e);

                // 如果是不可重试的错误，立即返回
                if !e.should_retry() {
                    return Err(e);
                }

                // 保存错误信息用于最后返回（不需要 Clone）
                let error_type = match &e {
                    TodoError::DatabaseError(msg) if msg.contains("pool") => {
                        "ConnectionPoolTimeout"
                    },
                    TodoError::DatabaseError(_) => "DatabaseError",
                    TodoError::DbError(_) => "DbError",
                    TodoError::Timeout(_) => "Timeout",
                    _ => "Other",
                };

                last_error = Some(e);

                // 如果还有剩余重试次数，等待后重试
                if attempt < config.max_retries {
                    let delay = config.delay_for_attempt(attempt);
                    info!(
                        "Retrying in {:?} (attempt {}/{}) - Error type: {}",
                        delay,
                        attempt + 1,
                        config.max_retries,
                        error_type
                    );
                    tokio::time::sleep(delay).await;
                }
            },
        }
    }

    Err(last_error.expect("retry loop exhausted without error"))
}
