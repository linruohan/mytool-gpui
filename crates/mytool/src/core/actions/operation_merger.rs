//! 操作合并器 - 减少快速连续操作导致的数据库压力
//!
//! 这个模块提供了操作合并和防抖机制：
//! 1. **防抖**: 收集快速连续的操作，等待一小段时间后再执行
//! 2. **合并**: 对于同一item的连续更新，只保留最新的
//!
//! ## 使用场景
//! - 用户快速打字时，不需要每次按键都触发数据库保存
//! - 用户拖动调整顺序时，只保存最终位置
//!
//! ## 示例
//! ```ignore
//! let merger = OperationMerger::new(300); // 300ms 防抖
//! merger.queue_update(item.clone());
//! ```

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::Duration,
};

use todos::entity::ItemModel;
use tokio::time::Sleep;
use tracing::{debug, warn};

/// 待处理的操作类型
#[derive(Debug, Clone)]
pub enum PendingOperation {
    /// 添加操作
    Add(Arc<ItemModel>),
    /// 更新操作
    Update(Arc<ItemModel>),
    /// 删除操作
    Delete(String),
}

/// 操作合并器配置
#[derive(Debug, Clone)]
pub struct OperationMergerConfig {
    /// 防抖延迟（毫秒）
    pub debounce_ms: u64,
    /// 最大待处理操作数
    pub max_pending: usize,
}

impl Default for OperationMergerConfig {
    fn default() -> Self {
        Self { debounce_ms: 300, max_pending: 100 }
    }
}

impl OperationMergerConfig {
    /// 创建新的配置
    pub fn new(debounce_ms: u64) -> Self {
        Self { debounce_ms, max_pending: 100 }
    }

    /// 创建高灵敏度配置（更快的响应，更频繁的保存）
    pub fn high_sensitivity() -> Self {
        Self { debounce_ms: 150, max_pending: 50 }
    }

    /// 创建低灵敏度配置（更少的保存，更长的等待）
    pub fn low_sensitivity() -> Self {
        Self { debounce_ms: 500, max_pending: 200 }
    }
}

/// 操作合并器
///
/// 收集快速连续的操作，应用防抖和合并策略，
/// 减少对数据库的访问次数。
pub struct OperationMerger {
    /// 待处理的操作队列（按 item ID 索引）
    pending_operations: BTreeMap<String, PendingOperation>,
    /// 追踪插入顺序（用于容量限制时移除最旧的操作）
    insertion_order: VecDeque<String>,
    /// 防抖计时器
    debounce_timer: Option<Sleep>,
    /// 配置
    config: OperationMergerConfig,
    /// 标记是否正在等待计时器
    is_timer_running: bool,
}

impl OperationMerger {
    /// 创建新的操作合并器
    pub fn new(config: OperationMergerConfig) -> Self {
        Self {
            pending_operations: BTreeMap::new(),
            insertion_order: VecDeque::new(),
            debounce_timer: None,
            config,
            is_timer_running: false,
        }
    }

    /// 创建使用默认配置的操作合并器
    pub fn with_default_config() -> Self {
        Self::new(OperationMergerConfig::default())
    }

    /// 创建高灵敏度配置的操作合并器
    pub fn with_high_sensitivity() -> Self {
        Self::new(OperationMergerConfig::high_sensitivity())
    }

    /// 清空所有待处理的操作
    pub fn clear(&mut self) {
        self.pending_operations.clear();
        self.insertion_order.clear();
        self.debounce_timer = None;
        self.is_timer_running = false;
    }

    /// 获取当前待处理操作的数量
    pub fn pending_count(&self) -> usize {
        self.pending_operations.len()
    }

    /// 检查是否有待处理的操作
    pub fn has_pending(&self) -> bool {
        !self.pending_operations.is_empty()
    }

    /// 获取所有待处理的添加操作
    pub fn get_pending_adds(&self) -> Vec<Arc<ItemModel>> {
        self.pending_operations
            .values()
            .filter_map(|op| match op {
                PendingOperation::Add(item) => Some(item.clone()),
                _ => None,
            })
            .collect()
    }

    /// 获取所有待处理的更新操作
    pub fn get_pending_updates(&self) -> Vec<Arc<ItemModel>> {
        self.pending_operations
            .values()
            .filter_map(|op| match op {
                PendingOperation::Update(item) => Some(item.clone()),
                _ => None,
            })
            .collect()
    }

    /// 获取所有待处理的删除操作
    pub fn get_pending_deletes(&self) -> Vec<String> {
        self.pending_operations
            .values()
            .filter_map(|op| match op {
                PendingOperation::Delete(id) => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    /// 排空所有待处理操作并返回
    ///
    /// 调用此方法后，内部队列会被清空
    pub fn drain_operations(&mut self) -> Vec<PendingOperation> {
        self.debounce_timer = None;
        self.is_timer_running = false;
        self.insertion_order.clear();
        let result: Vec<_> = self.pending_operations.values().map(|v| v.clone()).collect();
        self.pending_operations.clear();
        result
    }
}

impl OperationMerger {
    /// 队列化一个更新操作
    ///
    /// 如果同一 item ID 已存在待处理操作，会被合并（只保留最新的）
    /// 如果触发了防抖计时器，返回 true
    pub fn queue_update(&mut self, item: Arc<ItemModel>) -> bool {
        let item_id = item.id.clone();
        let is_new_key = !self.pending_operations.contains_key(&item_id);

        // 如果已达最大容量，记录警告并跳过最旧的操作
        if is_new_key && self.pending_operations.len() >= self.config.max_pending {
            warn!(
                "OperationMerger: reached max capacity ({}), dropping oldest operation",
                self.config.max_pending
            );
            // 从插入顺序队列中移除最旧的操作
            if let Some(oldest_key) = self.insertion_order.pop_front() {
                self.pending_operations.remove(&oldest_key);
            }
        }

        // 合并操作：只保留最新的更新
        let should_restart_timer = match self.pending_operations.get(&item_id) {
            // 如果已存在相同 item 的操作，检查是否需要升级为 Update（Add->Update
            // 可能是新建后立即修改）
            Some(existing) => {
                debug!(
                    "OperationMerger: merging update for item {}, existing op: {:?}",
                    item_id, existing
                );
                match existing {
                    PendingOperation::Add(_) => {
                        // 已有 Add，替换为 Update
                        self.pending_operations
                            .insert(item_id.clone(), PendingOperation::Update(item));
                        false // 不重启计时器，保持原来的计时
                    },
                    PendingOperation::Update(_) => {
                        // 已有 Update，直接替换
                        self.pending_operations
                            .insert(item_id.clone(), PendingOperation::Update(item));
                        true // 更新已存在，应该重启计时器
                    },
                    PendingOperation::Delete(_) => {
                        // 之前被删除了，现在又添加，应该替换为 Add
                        self.pending_operations
                            .insert(item_id.clone(), PendingOperation::Add(item));
                        // 如果之前被删除，需要重新添加到插入顺序
                        if !self.insertion_order.contains(&item_id) {
                            self.insertion_order.push_back(item_id.clone());
                        }
                        true
                    },
                }
            },
            None => {
                // 新操作
                debug!("OperationMerger: queuing new update for item {}", item_id);
                self.pending_operations.insert(item_id.clone(), PendingOperation::Update(item));
                self.insertion_order.push_back(item_id.clone());
                true
            },
        };

        // 启动或重启防抖计时器
        if should_restart_timer && !self.is_timer_running {
            self.debounce_timer =
                Some(tokio::time::sleep(Duration::from_millis(self.config.debounce_ms)));
            self.is_timer_running = true;
            return true;
        }

        should_restart_timer
    }

    /// 队列化一个添加操作
    ///
    /// 如果同一 item ID 已存在，会被合并
    pub fn queue_add(&mut self, item: Arc<ItemModel>) -> bool {
        let item_id = item.id.clone();
        let is_new_key = !self.pending_operations.contains_key(&item_id);

        // 如果已达最大容量且是新 key，移除最旧的操作
        if is_new_key && self.pending_operations.len() >= self.config.max_pending {
            if let Some(oldest_key) = self.insertion_order.pop_front() {
                self.pending_operations.remove(&oldest_key);
            }
        }

        match self.pending_operations.get(&item_id) {
            Some(existing) => {
                debug!(
                    "OperationMerger: merging add for item {}, existing op: {:?}",
                    item_id, existing
                );
                // 已有操作，都统一为 Update（以最新的数据为准）
                self.pending_operations.insert(item_id, PendingOperation::Update(item));
                true
            },
            None => {
                debug!("OperationMerger: queuing new add for item {}", item_id);
                self.pending_operations.insert(item_id.clone(), PendingOperation::Add(item));
                self.insertion_order.push_back(item_id);
                true
            },
        }
    }

    /// 队列化一个删除操作
    ///
    /// 如果同一 item ID 已存在添加/更新操作，会被移除并替换为 Delete
    pub fn queue_delete(&mut self, item_id: String) -> bool {
        // 如果有待处理的 Add/Update，移除它们
        if let Some(existing) = self.pending_operations.remove(&item_id) {
            debug!(
                "OperationMerger: removing pending op {:?} for deleted item {}",
                existing, item_id
            );
            // 从插入顺序中移除
            self.insertion_order.retain(|k| k != &item_id);
        }

        // 添加 Delete 操作
        debug!("OperationMerger: queuing delete for item {}", item_id);
        self.pending_operations.insert(item_id.clone(), PendingOperation::Delete(item_id.clone()));
        self.insertion_order.push_back(item_id);
        true
    }

    /// 检查防抖计时器是否到期
    ///
    /// 如果计时器已到期，返回已到期的计时器，可用于 await
    pub fn check_timer(&mut self) -> Option<&mut Sleep> {
        if self.is_timer_running {
            self.debounce_timer.as_mut();
            // 注意：这个方法不等待，只是返回引用
            // 调用者需要手动 poll 或 await
            self.debounce_timer.as_mut()
        } else {
            None
        }
    }

    /// 消费计时器（用于 await）
    ///
    /// 如果计时器正在运行，返回一个可等待的 Sleep future
    pub fn take_timer(&mut self) -> Option<Sleep> {
        if self.is_timer_running {
            self.is_timer_running = false;
            self.debounce_timer.take()
        } else {
            None
        }
    }

    /// 取消计时器
    pub fn cancel_timer(&mut self) {
        self.debounce_timer = None;
        self.is_timer_running = false;
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

    #[tokio::test]
    async fn test_queue_update_merges_same_item() {
        let mut merger = OperationMerger::with_default_config();

        let item1 = Arc::new(create_test_item("1"));
        let item2 = Arc::new(create_test_item("1"));

        merger.queue_add(item1);
        assert_eq!(merger.pending_count(), 1);

        // 再次添加相同ID，应该合并
        merger.queue_add(item2);
        assert_eq!(merger.pending_count(), 1);

        // 更新不同ID
        merger.queue_update(Arc::new(create_test_item("2")));
        assert_eq!(merger.pending_count(), 2);
    }

    #[test]
    fn test_queue_delete_removes_pending() {
        let mut merger = OperationMerger::with_default_config();

        merger.queue_add(Arc::new(create_test_item("1")));
        assert_eq!(merger.pending_count(), 1);

        merger.queue_delete("1".to_string());
        assert_eq!(merger.pending_count(), 1); // Delete 替代了 Add

        let ops = merger.drain_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PendingOperation::Delete(id) => assert_eq!(id, "1"),
            _ => panic!("Expected Delete operation"),
        }
    }

    #[tokio::test]
    async fn test_drain_operations() {
        let mut merger = OperationMerger::with_default_config();

        merger.queue_add(Arc::new(create_test_item("1")));
        merger.queue_update(Arc::new(create_test_item("2")));
        merger.queue_delete("3".to_string());

        assert_eq!(merger.pending_count(), 3);

        let ops = merger.drain_operations();
        assert_eq!(ops.len(), 3);
        assert!(!merger.has_pending());
    }

    #[test]
    fn test_max_capacity() {
        let config = OperationMergerConfig { debounce_ms: 300, max_pending: 2 };
        let mut merger = OperationMerger::new(config);

        merger.queue_add(Arc::new(create_test_item("1")));
        merger.queue_add(Arc::new(create_test_item("2")));
        assert_eq!(merger.pending_count(), 2);

        // 超过容量，最旧的会被移除
        merger.queue_add(Arc::new(create_test_item("3")));
        assert_eq!(merger.pending_count(), 2);

        // 检查最旧的 "1" 被移除了
        assert!(merger.pending_operations.get("1").is_none());
    }
}
