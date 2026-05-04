//! 操作队列 - 管理异步操作的顺序执行和冲突检测
//!
//! 这个模块提供了操作队列的实现，用于：
//!
//! ## 功能
//! 1. **操作序列化** - 保证操作的顺序执行，避免race condition
//! 2. **版本控制** - 每个操作带有版本号，用于检测冲突
//! 3. **冲突处理** - 检测并发修改，提供处理策略
//!
//! ## 设计理念
//! - 使用channel作为队列，异步执行操作
//! - 每个操作带有预期的版本号（基于乐观锁）
//! - 检测到冲突时提供多种处理策略
//!
//! ## 使用示例
//! ```ignore
//! let queue = OperationQueue::new();
//!
//! // 提交操作
//! queue.submit(OpCommand::Update { id, data, expected_version: 5 })
//!     .await;
//!
//! // 检查队列状态
//! if queue.has_pending() {
//!     // 有操作正在进行中
//! }
//! ```

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use todos::entity::ItemModel;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// 操作命令 - 需要执行的操作
#[derive(Debug, Clone)]
pub enum OpCommand {
    /// 添加操作
    Add { item: Arc<ItemModel> },
    /// 更新操作
    Update { id: String, item: Arc<ItemModel>, expected_version: usize },
    /// 删除操作
    Delete { id: String, expected_version: usize },
    /// 批量操作
    Batch { commands: Vec<OpCommand> },
}

impl OpCommand {
    /// 获取操作关联的item ID（如果有）
    pub fn item_id(&self) -> Option<&str> {
        match self {
            OpCommand::Add { item } => Some(&item.id),
            OpCommand::Update { id, .. } => Some(id),
            OpCommand::Delete { id, .. } => Some(id),
            OpCommand::Batch { commands } => commands.first().and_then(|c| c.item_id()),
        }
    }

    /// 获取操作的预期版本号
    pub fn expected_version(&self) -> Option<usize> {
        match self {
            OpCommand::Add { .. } => None,
            OpCommand::Update { expected_version, .. } => Some(*expected_version),
            OpCommand::Delete { expected_version, .. } => Some(*expected_version),
            OpCommand::Batch { commands } => commands.first().and_then(|c| c.expected_version()),
        }
    }
}

/// 冲突处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictStrategy {
    /// 最后写入胜出（Last-Write-Wins）- 使用最新数据覆盖
    #[default]
    LastWriteWins,
    /// 第一个写入胜出（First-Write-Wins）- 保留原始数据
    FirstWriteWins,
    /// 手动解决 - 返回冲突供调用者处理
    Manual,
}

/// 冲突信息
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// 冲突的操作
    pub operation: OpCommand,
    /// 当前数据库中的版本
    pub current_version: usize,
    /// 操作预期的版本
    pub expected_version: usize,
    /// 当前数据库中的item
    pub current_item: Option<Arc<ItemModel>>,
}

impl ConflictInfo {
    /// 创建一个新的冲突信息
    pub fn new(
        operation: OpCommand,
        current_version: usize,
        current_item: Option<Arc<ItemModel>>,
    ) -> Self {
        Self {
            current_version,
            expected_version: operation.expected_version().unwrap_or(0),
            operation,
            current_item,
        }
    }

    /// 检查是否是版本冲突
    pub fn is_version_conflict(&self) -> bool {
        self.current_version != self.expected_version
    }

    /// 生成用户友好的冲突描述
    pub fn describe_for_user(&self) -> String {
        let item_id = self.operation.item_id().unwrap_or("unknown");
        match self.operation {
            OpCommand::Update { .. } => {
                format!(
                    "任务(ID: {})已被其他操作修改。\n您的版本: {}, 当前版本: {}",
                    item_id, self.expected_version, self.current_version
                )
            },
            OpCommand::Delete { .. } => {
                format!(
                    "任务(ID: {})不存在或已被删除。\n预期版本: {}, 当前版本: {}",
                    item_id, self.expected_version, self.current_version
                )
            },
            _ => format!("任务(ID: {})发生冲突", item_id),
        }
    }
}

/// 操作结果
#[derive(Debug)]
pub enum OpResult {
    /// 操作成功
    Success,
    /// 操作失败
    Failed(String),
    /// 操作因冲突失败
    Conflict(ConflictInfo),
    /// 操作被取消
    Cancelled,
}

/// 操作元数据
#[derive(Debug, Clone)]
pub struct OperationMeta {
    /// 操作用于恢复的完整item数据（用于删除等操作）
    pub recovery_data: Option<Arc<ItemModel>>,
    /// 冲突处理策略
    pub conflict_strategy: ConflictStrategy,
    /// 操作描述（用于日志）
    pub description: String,
}

impl OperationMeta {
    pub fn new(description: &str) -> Self {
        Self {
            recovery_data: None,
            conflict_strategy: ConflictStrategy::LastWriteWins,
            description: description.to_string(),
        }
    }

    pub fn with_recovery(mut self, item: Arc<ItemModel>) -> Self {
        self.recovery_data = Some(item);
        self
    }

    pub fn with_strategy(mut self, strategy: ConflictStrategy) -> Self {
        self.conflict_strategy = strategy;
        self
    }
}

/// 操作状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    /// 队列空闲
    Idle,
    /// 队列正在处理操作
    Processing,
    /// 队列已暂停（遇到冲突）
    Paused,
    /// 队列已停止
    Stopped,
}

/// 操作队列状态
#[derive(Debug, Clone)]
pub struct QueueState {
    /// 队列状态
    pub status: QueueStatus,
    /// 待处理操作数
    pub pending_count: usize,
    /// 已完成操作数
    pub completed_count: usize,
    /// 失败操作数
    pub failed_count: usize,
    /// 冲突操作数
    pub conflict_count: usize,
    /// 最后一次操作的时间戳
    pub last_operation_time: Option<std::time::Instant>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            status: QueueStatus::Idle,
            pending_count: 0,
            completed_count: 0,
            failed_count: 0,
            conflict_count: 0,
            last_operation_time: None,
        }
    }
}

/// 操作队列
///
/// 管理异步操作的顺序执行和冲突检测
pub struct OperationQueue {
    /// 命令发送通道
    cmd_tx: mpsc::Sender<OpCommand>,
    /// 元数据发送通道
    meta_tx: mpsc::Sender<OperationMeta>,
    /// 状态
    state: Arc<QueueStateInner>,
}

#[derive(Debug)]
struct QueueStateInner {
    status: std::sync::Mutex<QueueStatus>,
    pending_count: AtomicUsize,
    completed_count: AtomicUsize,
    failed_count: AtomicUsize,
    conflict_count: AtomicUsize,
    is_shutdown: AtomicBool,
}

impl QueueStateInner {
    fn new() -> Self {
        Self {
            status: std::sync::Mutex::new(QueueStatus::Idle),
            pending_count: AtomicUsize::new(0),
            completed_count: AtomicUsize::new(0),
            failed_count: AtomicUsize::new(0),
            conflict_count: AtomicUsize::new(0),
            is_shutdown: AtomicBool::new(false),
        }
    }

    fn set_status(&self, status: QueueStatus) {
        *self.status.lock().unwrap() = status;
    }

    fn get_status(&self) -> QueueStatus {
        *self.status.lock().unwrap()
    }
}

impl OperationQueue {
    /// 创建新的操作队列
    pub fn new() -> Self {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<OpCommand>(100);
        let (meta_tx, _meta_rx) = mpsc::channel::<OperationMeta>(100);

        Self { cmd_tx, meta_tx, state: Arc::new(QueueStateInner::new()) }
    }

    /// 获取队列状态
    pub fn state(&self) -> QueueState {
        QueueState {
            status: self.state.get_status(),
            pending_count: self.state.pending_count.load(Ordering::Relaxed),
            completed_count: self.state.completed_count.load(Ordering::Relaxed),
            failed_count: self.state.failed_count.load(Ordering::Relaxed),
            conflict_count: self.state.conflict_count.load(Ordering::Relaxed),
            last_operation_time: None,
        }
    }

    /// 检查队列是否正在处理操作
    pub fn is_processing(&self) -> bool {
        self.state.get_status() == QueueStatus::Processing
    }

    /// 检查队列是否有待处理的操作
    pub fn has_pending(&self) -> bool {
        self.state.pending_count.load(Ordering::Relaxed) > 0
    }

    /// 提交操作到队列
    pub async fn submit(&self, command: OpCommand, meta: OperationMeta) -> Result<(), String> {
        if self.state.is_shutdown.load(Ordering::Relaxed) {
            return Err("Queue is shutdown".to_string());
        }

        self.state.pending_count.fetch_add(1, Ordering::Relaxed);
        self.state.set_status(QueueStatus::Processing);

        self.cmd_tx.send(command).await.map_err(|e| format!("Failed to send command: {}", e))?;

        self.meta_tx.send(meta).await.map_err(|e| format!("Failed to send meta: {}", e))?;

        debug!("Operation submitted to queue");
        Ok(())
    }

    /// 取消所有待处理的操作
    pub fn cancel_pending(&self) {
        warn!("Cancelling all pending operations");
        self.state.pending_count.store(0, Ordering::Relaxed);
        self.state.set_status(QueueStatus::Idle);
    }

    /// 关闭队列
    pub fn shutdown(&self) {
        info!("Shutting down operation queue");
        self.state.is_shutdown.store(true, Ordering::Relaxed);
        self.state.set_status(QueueStatus::Stopped);
    }

    /// 记录操作完成
    pub fn record_complete(&self) {
        self.state.pending_count.fetch_sub(1, Ordering::Relaxed);
        self.state.completed_count.fetch_add(1, Ordering::Relaxed);

        if self.state.pending_count.load(Ordering::Relaxed) == 0 {
            self.state.set_status(QueueStatus::Idle);
        }

        debug!(
            "Operation completed. Total: {}, Failed: {}, Conflicts: {}",
            self.state.completed_count.load(Ordering::Relaxed),
            self.state.failed_count.load(Ordering::Relaxed),
            self.state.conflict_count.load(Ordering::Relaxed)
        );
    }

    /// 记录操作失败
    pub fn record_failed(&self) {
        self.state.pending_count.fetch_sub(1, Ordering::Relaxed);
        self.state.failed_count.fetch_add(1, Ordering::Relaxed);

        if self.state.pending_count.load(Ordering::Relaxed) == 0 {
            self.state.set_status(QueueStatus::Idle);
        }
    }

    /// 记录冲突
    pub fn record_conflict(&self) {
        self.state.pending_count.fetch_sub(1, Ordering::Relaxed);
        self.state.conflict_count.fetch_add(1, Ordering::Relaxed);
        self.state.set_status(QueueStatus::Paused);
    }

    /// 获取统计信息
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.state.pending_count.load(Ordering::Relaxed),
            completed: self.state.completed_count.load(Ordering::Relaxed),
            failed: self.state.failed_count.load(Ordering::Relaxed),
            conflicts: self.state.conflict_count.load(Ordering::Relaxed),
            is_shutdown: self.state.is_shutdown.load(Ordering::Relaxed),
        }
    }
}

impl Default for OperationQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 队列统计信息
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    pub conflicts: usize,
    pub is_shutdown: bool,
}

impl QueueStats {
    pub fn is_healthy(&self) -> bool {
        !self.is_shutdown && self.failed == 0 && self.conflicts == 0
    }

    pub fn total_processed(&self) -> usize {
        self.completed + self.failed + self.conflicts
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_processed();
        if total == 0 {
            return 1.0;
        }
        self.completed as f64 / total as f64
    }
}

/// 操作监控器 - 用于监控操作队列的状态
#[allow(dead_code)]
pub struct OperationMonitor {
    queue: Arc<QueueStateInner>,
    history: Mutex<VecDeque<HistoryEntry>>,
    max_history: usize,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    command: String,
    result: String,
    timestamp: std::time::Instant,
}

impl OperationMonitor {
    pub fn new(queue: &OperationQueue) -> Self {
        Self { queue: queue.state.clone(), history: Mutex::new(VecDeque::new()), max_history: 50 }
    }

    pub fn record(&self, command_type: &str, result: &str) {
        let mut history = self.history.lock().unwrap();
        history.push_back(HistoryEntry {
            command: command_type.to_string(),
            result: result.to_string(),
            timestamp: std::time::Instant::now(),
        });

        while history.len() > self.max_history {
            history.pop_front();
        }
    }

    pub fn get_recent_history(&self, count: usize) -> Vec<String> {
        let history = self.history.lock().unwrap();
        history
            .iter()
            .rev()
            .take(count)
            .map(|e| format!("[{:?}] {} - {}", e.timestamp.elapsed(), e.command, e.result))
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
    fn test_op_command_item_id() {
        let add_cmd = OpCommand::Add { item: Arc::new(create_test_item("1")) };
        assert_eq!(add_cmd.item_id(), Some("1"));

        let update_cmd = OpCommand::Update {
            id: "2".to_string(),
            item: Arc::new(create_test_item("2")),
            expected_version: 5,
        };
        assert_eq!(update_cmd.item_id(), Some("2"));

        let delete_cmd = OpCommand::Delete { id: "3".to_string(), expected_version: 3 };
        assert_eq!(delete_cmd.item_id(), Some("3"));
    }

    #[test]
    fn test_conflict_info() {
        let operation = OpCommand::Update {
            id: "1".to_string(),
            item: Arc::new(create_test_item("1")),
            expected_version: 5,
        };

        let conflict = ConflictInfo::new(operation, 7, None);

        assert!(conflict.is_version_conflict());
        assert_eq!(conflict.expected_version, 5);
        assert_eq!(conflict.current_version, 7);
    }

    #[test]
    fn test_queue_stats() {
        let stats =
            QueueStats { pending: 2, completed: 10, failed: 1, conflicts: 1, is_shutdown: false };

        assert!(!stats.is_healthy()); // 因为有 conflicts
        assert_eq!(stats.total_processed(), 12);
        assert!((stats.success_rate() - 0.833).abs() < 0.01);
    }
}
