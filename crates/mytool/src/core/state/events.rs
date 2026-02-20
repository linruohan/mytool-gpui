//! 事件总线系统 - 细粒度的状态变化通知
//!
//! 这个模块提供了一个事件总线，用于在数据变化时发送细粒度的通知，
//! 避免全局观察者导致的不必要重新渲染。

use std::sync::Arc;

use gpui::Global;
use todos::entity::ItemModel;

/// TodoStore 事件类型
#[derive(Debug, Clone)]
pub enum TodoStoreEvent {
    /// 任务被添加（只传递 ID，避免大量数据复制）
    ItemAdded(String),
    /// 任务被更新
    ItemUpdated(String),
    /// 任务被删除
    ItemDeleted(String),
    /// 项目变化
    ProjectChanged(String),
    /// 批量更新（需要全量刷新）
    BulkUpdate,
    /// 活跃项目变化
    ActiveProjectChanged,
}

/// 事件总线
///
/// 用于发布和订阅 TodoStore 的变化事件
pub struct TodoEventBus {
    /// 事件历史（用于调试和审计）
    event_history: Vec<TodoStoreEvent>,
    /// 最大历史记录数
    max_history: usize,
}

impl Global for TodoEventBus {}

impl TodoEventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        Self { event_history: Vec::new(), max_history: 100 }
    }

    /// 发布事件
    pub fn publish(&mut self, event: TodoStoreEvent) {
        // 记录事件历史
        self.event_history.push(event.clone());

        // 限制历史记录大小
        if self.event_history.len() > self.max_history {
            self.event_history.remove(0);
        }

        // 在实际应用中，这里会通知所有订阅者
        // 由于 GPUI 的观察者模式，我们通过 cx.notify() 来触发更新
    }

    /// 获取最近的事件
    pub fn recent_events(&self, count: usize) -> &[TodoStoreEvent] {
        let start = self.event_history.len().saturating_sub(count);
        &self.event_history[start..]
    }

    /// 清空事件历史
    pub fn clear_history(&mut self) {
        self.event_history.clear();
    }
}

impl Default for TodoEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 批量操作队列
///
/// 用于收集和批量处理操作，减少数据库访问次数
pub struct BatchOperations {
    /// 待添加的任务
    pub pending_adds: Vec<Arc<ItemModel>>,
    /// 待更新的任务
    pub pending_updates: Vec<Arc<ItemModel>>,
    /// 待删除的任务 ID
    pub pending_deletes: Vec<String>,
    /// 是否有待处理的操作
    pub has_pending: bool,
}

impl Global for BatchOperations {}

impl BatchOperations {
    /// 创建新的批量操作队列
    pub fn new() -> Self {
        Self {
            pending_adds: Vec::new(),
            pending_updates: Vec::new(),
            pending_deletes: Vec::new(),
            has_pending: false,
        }
    }

    /// 添加待添加的任务
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.pending_adds.push(item);
        self.has_pending = true;
    }

    /// 添加待更新的任务
    pub fn update_item(&mut self, item: Arc<ItemModel>) {
        self.pending_updates.push(item);
        self.has_pending = true;
    }

    /// 添加待删除的任务
    pub fn delete_item(&mut self, id: String) {
        self.pending_deletes.push(id);
        self.has_pending = true;
    }

    /// 清空所有待处理的操作
    pub fn clear(&mut self) {
        self.pending_adds.clear();
        self.pending_updates.clear();
        self.pending_deletes.clear();
        self.has_pending = false;
    }

    /// 获取待处理操作的数量
    pub fn pending_count(&self) -> usize {
        self.pending_adds.len() + self.pending_updates.len() + self.pending_deletes.len()
    }
}

impl Default for BatchOperations {
    fn default() -> Self {
        Self::new()
    }
}
