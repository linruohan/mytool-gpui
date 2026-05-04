//! 操作结果追踪 - 追踪异步操作的结果并提供错误恢复
//!
//! 这个模块提供了操作结果的追踪和错误恢复机制：
//!
//! ## 功能
//! 1. **结果追踪**: 记录每个操作的结果（成功/失败/重试中）
//! 2. **错误通知**: 将失败的操作信息通知到UI层
//! 3. **自动恢复**: 对于可恢复的错误，提供恢复机制
//!
//! ## 使用场景
//! ```ignore
//! let tracker = OperationResultTracker::new();
//!
//! // 追踪一个操作
//! tracker.track(item_id.clone(), OperationStatus::Pending);
//!
//! // 操作完成后更新状态
//! tracker.complete(&item_id, Ok(()));
//!
//! // 或者标记失败
//! tracker.fail(&item_id, Err(e));
//!
//! // 检查是否有失败的 操作
//! if let Some(failed) = tracker.get_failed_operations() {
//!     // 通知用户或自动重试
//! }
//! ```

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Instant,
};

use todos::entity::ItemModel;
use tracing::{debug, info, warn};

/// 操作状态
#[derive(Debug, Clone)]
pub enum OperationStatus {
    /// 操作正在等待（队列中）
    Pending,
    /// 操作正在执行
    InProgress,
    /// 操作成功完成
    Success,
    /// 操作失败
    Failed { error: String, retry_count: usize, last_attempt: Instant },
    /// 操作已提交到UI待处理（用于add操作）
    SubmittedToUI,
}

/// 失败操作的详细信息
#[derive(Debug, Clone)]
pub struct FailedOperation {
    /// 操作类型
    pub operation_type: OperationType,
    /// 关联的item ID
    pub item_id: String,
    /// 错误消息
    pub error: String,
    /// 原始item数据（用于恢复）
    pub item_data: Option<Arc<ItemModel>>,
    /// 重试次数
    pub retry_count: usize,
    /// 首次失败时间
    pub first_failure: Instant,
    /// 最后一次尝试时间
    pub last_attempt: Instant,
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Add,
    Update,
    Delete,
    Complete,
    Pin,
    Unpin,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Add => write!(f, "添加"),
            OperationType::Update => write!(f, "更新"),
            OperationType::Delete => write!(f, "删除"),
            OperationType::Complete => write!(f, "完成"),
            OperationType::Pin => write!(f, "置顶"),
            OperationType::Unpin => write!(f, "取消置顶"),
        }
    }
}

/// 操作结果追踪器
///
/// 追踪所有异步操作的状态，提供错误检测和恢复能力
pub struct OperationResultTracker {
    /// 操作状态映射（item_id -> 状态）
    operations: HashMap<String, OperationStatus>,
    /// 操作历史（用于调试和显示）
    history: VecDeque<OperationHistoryEntry>,
    /// 最大历史记录数
    max_history: usize,
    /// 待恢复的操作（用于UI通知）
    pending_recoveries: Vec<FailedOperation>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct OperationHistoryEntry {
    item_id: String,
    operation_type: OperationType,
    status: OperationStatus,
    timestamp: Instant,
}

impl OperationResultTracker {
    /// 创建新的追踪器
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
            history: VecDeque::new(),
            max_history: 100,
            pending_recoveries: Vec::new(),
        }
    }

    /// 追踪一个新操作
    pub fn track(&mut self, item_id: String, op_type: OperationType) {
        debug!("Tracking {} operation for item: {}", op_type, item_id);
        self.operations.insert(item_id.clone(), OperationStatus::Pending);
        self.add_history(item_id, op_type, OperationStatus::Pending);
    }

    /// 标记操作为进行中
    pub fn mark_in_progress(&mut self, item_id: &str) {
        if let Some(status) = self.operations.get_mut(item_id) {
            *status = OperationStatus::InProgress;
        }
    }

    /// 标记操作成功完成
    pub fn complete(&mut self, item_id: &str, op_type: OperationType) {
        info!("Operation completed for item: {}", item_id);
        self.operations.insert(item_id.to_string(), OperationStatus::Success);
        self.add_history(item_id.to_string(), op_type, OperationStatus::Success);

        // 从待恢复列表中移除
        self.pending_recoveries.retain(|f| f.item_id != item_id);
    }

    /// 标记操作失败
    ///
    /// # 参数
    /// - `item_id`: 操作关联的item ID
    /// - `op_type`: 操作类型
    /// - `error`: 错误消息
    /// - `item_data`: 可选的原始item数据（用于恢复）
    /// - `retry_count`: 当前重试次数
    pub fn fail(
        &mut self,
        item_id: &str,
        op_type: OperationType,
        error: String,
        item_data: Option<Arc<ItemModel>>,
        retry_count: usize,
    ) {
        warn!(
            "Operation failed for item: {}, error: {}, retry_count: {}",
            item_id, error, retry_count
        );

        let now = Instant::now();
        let status =
            OperationStatus::Failed { error: error.clone(), retry_count, last_attempt: now };

        self.operations.insert(item_id.to_string(), status.clone());
        self.add_history(item_id.to_string(), op_type, status);

        // 添加到待恢复列表
        let failed_op = FailedOperation {
            operation_type: op_type,
            item_id: item_id.to_string(),
            error,
            item_data,
            retry_count,
            first_failure: now,
            last_attempt: now,
        };

        // 避免重复添加
        self.pending_recoveries.retain(|f| f.item_id != item_id);
        self.pending_recoveries.push(failed_op);
    }

    /// 标记操作已提交到UI（用于add操作，临时ID替换后）
    pub fn mark_submitted_to_ui(&mut self, item_id: &str) {
        if let Some(status) = self.operations.get_mut(item_id) {
            *status = OperationStatus::SubmittedToUI;
        }
    }

    /// 获取失败的操作列表
    pub fn get_failed_operations(&self) -> Vec<&FailedOperation> {
        self.pending_recoveries
            .iter()
            .filter(|f| f.retry_count == 0) // 只返回尚未重试的
            .collect()
    }

    /// 获取所有待恢复的操作（用于UI通知）
    pub fn get_pending_recoveries(&self) -> &Vec<FailedOperation> {
        &self.pending_recoveries
    }

    /// 移除已恢复的操作
    pub fn remove_recovery(&mut self, item_id: &str) {
        self.pending_recoveries.retain(|f| f.item_id != item_id);
        self.operations.remove(item_id);
    }

    /// 获取操作状态
    pub fn get_status(&self, item_id: &str) -> Option<&OperationStatus> {
        self.operations.get(item_id)
    }

    /// 检查是否有正在进行的操作
    pub fn has_pending_operations(&self) -> bool {
        self.operations.values().any(|status| match status {
            OperationStatus::Pending | OperationStatus::InProgress => true,
            _ => false,
        })
    }

    /// 检查是否有失败的操作
    pub fn has_failed_operations(&self) -> bool {
        !self.pending_recoveries.is_empty()
    }

    /// 获取失败操作数量
    pub fn failed_count(&self) -> usize {
        self.pending_recoveries.len()
    }

    /// 获取正在进行的操作数量
    pub fn pending_count(&self) -> usize {
        self.operations
            .values()
            .filter(|s| matches!(s, OperationStatus::Pending | OperationStatus::InProgress))
            .count()
    }

    /// 清空已完成和成功的操作记录
    pub fn cleanup(&mut self) {
        // 移除成功状态的操作
        self.operations.retain(|_, status| !matches!(status, OperationStatus::Success));

        // 限制历史记录大小
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// 获取操作历史（最近的在前）
    pub fn get_history(&self) -> Vec<&OperationHistoryEntry> {
        self.history.iter().rev().collect()
    }

    fn add_history(&mut self, item_id: String, op_type: OperationType, status: OperationStatus) {
        self.history.push_back(OperationHistoryEntry {
            item_id,
            operation_type: op_type,
            status,
            timestamp: Instant::now(),
        });

        // 限制历史大小
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }
}

impl Default for OperationResultTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationResultTracker {
    /// 生成用户友好的错误消息
    pub fn format_error_for_user(&self, failed: &FailedOperation) -> String {
        let action = match failed.operation_type {
            OperationType::Add => "添加任务",
            OperationType::Update => "更新任务",
            OperationType::Delete => "删除任务",
            OperationType::Complete => "完成任务",
            OperationType::Pin => "置顶任务",
            OperationType::Unpin => "取消置顶任务",
        };

        if failed.retry_count > 0 {
            format!(
                "{}失败（尝试重试 {}/3）：{}\n请检查网络连接后重试。",
                action, failed.retry_count, failed.error
            )
        } else {
            format!("{}失败：{}\n请稍后重试。", action, failed.error)
        }
    }

    /// 检查错误是否可重试
    pub fn is_retryable_error(&self, error: &str) -> bool {
        // 数据库锁定、超时等临时性错误可以重试
        let retryable_patterns = [
            "database is locked",
            "timeout",
            "connection",
            "temporary failure",
            "too many connections",
        ];

        retryable_patterns.iter().any(|pattern| error.to_lowercase().contains(pattern))
    }

    /// 获取应该重试的操作
    pub fn get_operations_to_retry(&self) -> Vec<&FailedOperation> {
        self.pending_recoveries
            .iter()
            .filter(|f| f.retry_count < 3 && self.is_retryable_error(&f.error))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(id: &str) -> ItemModel {
        ItemModel {
            id: id.to_string(),
            content: format!("Test item {}", id),
            checked: false,
            is_deleted: false,
            collapsed: false,
            pinned: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_track_operation() {
        let mut tracker = OperationResultTracker::new();

        tracker.track("item1".to_string(), OperationType::Add);
        assert!(tracker.has_pending_operations());

        let status = tracker.get_status("item1");
        assert!(matches!(status, Some(OperationStatus::Pending)));
    }

    #[test]
    fn test_complete_operation() {
        let mut tracker = OperationResultTracker::new();

        tracker.track("item1".to_string(), OperationType::Add);
        tracker.complete("item1", OperationType::Add);

        let status = tracker.get_status("item1");
        assert!(matches!(status, Some(OperationStatus::Success)));
    }

    #[test]
    fn test_fail_operation() {
        let mut tracker = OperationResultTracker::new();

        tracker.track("item1".to_string(), OperationType::Update);
        tracker.fail("item1", OperationType::Update, "Database error".to_string(), None, 0);

        assert!(tracker.has_failed_operations());
        assert_eq!(tracker.failed_count(), 1);

        let failed = tracker.get_failed_operations();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].item_id, "item1");
    }

    #[test]
    fn test_retry_logic() {
        let tracker = OperationResultTracker::new();

        // 可重试的错误
        assert!(tracker.is_retryable_error("database is locked"));
        assert!(tracker.is_retryable_error("timeout occurred"));

        // 不可重试的错误
        assert!(!tracker.is_retryable_error("item not found"));
        assert!(!tracker.is_retryable_error("validation error"));
    }

    #[test]
    fn test_format_error() {
        let tracker = OperationResultTracker::new();
        let failed = FailedOperation {
            operation_type: OperationType::Update,
            item_id: "item1".to_string(),
            error: "database is locked".to_string(),
            item_data: None,
            retry_count: 0,
            first_failure: Instant::now(),
            last_attempt: Instant::now(),
        };

        let msg = tracker.format_error_for_user(&failed);
        assert!(msg.contains("更新任务失败"));
        assert!(msg.contains("database is locked"));
    }
}
