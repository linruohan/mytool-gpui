use std::sync::Arc;

use futures::future::BoxFuture;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, TransactionTrait};
use tokio::time;

#[derive(Debug)]
pub struct TransactionManager {
    db: Arc<DatabaseConnection>,
}

impl TransactionManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn execute<T, F>(&self, operation: F) -> Result<T, DbErr>
    where
        F: Fn(&DatabaseConnection) -> futures::future::BoxFuture<'_, Result<T, DbErr>> + Send,
    {
        // 直接在数据库连接上执行操作，不使用事务
        // 注意：这不是一个真正的事务实现，只是为了编译通过
        // 后续需要重新设计事务管理逻辑
        operation(&self.db).await
    }

    pub async fn execute_with_retry<T, F>(
        &self,
        operation: F,
        max_retries: usize,
    ) -> Result<T, DbErr>
    where
        F: Fn(&DatabaseConnection) -> futures::future::BoxFuture<'_, Result<T, DbErr>>
            + Send
            + Clone,
    {
        let mut last_error: Option<DbErr> = None;

        for attempt in 0..max_retries {
            match self.execute(operation.clone()).await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!("Transaction succeeded on attempt {}", attempt + 1);
                    }
                    return Ok(result);
                },
                Err(e) => {
                    // 检查是否是可以重试的错误
                    if self.is_retryable_error(&e) {
                        if attempt < max_retries - 1 {
                            tracing::warn!(
                                "Transaction failed, retrying ({}/{})...: {:?}",
                                attempt + 1,
                                max_retries,
                                e
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                1000 * (attempt + 1) as u64,
                            ))
                            .await;
                            last_error = Some(e);
                        } else {
                            tracing::error!("All transaction attempts failed: {:?}", e);
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                },
            }
        }

        Err(last_error
            .unwrap_or_else(|| DbErr::Custom("Transaction failed with unknown error".to_string())))
    }

    fn is_retryable_error(&self, error: &DbErr) -> bool {
        // 检查是否是可以重试的错误类型
        match error {
            DbErr::Custom(msg) => {
                msg.contains("busy")
                    || msg.contains("locked")
                    || msg.contains("transaction")
                    || msg.contains("connection")
            },
            _ => false,
        }
    }
}
