//! 细粒度观察者系统 - 解决过度订阅问题
//!
//! 这个模块提供了一个细粒度的观察者系统，只在相关数据变化时通知视图，
//! 避免全局观察者导致的不必要重新渲染。

use std::sync::Arc;

use gpui::Global;
use todos::entity::ItemModel;

use super::TodoStoreEvent;

/// 视图类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewType {
    /// 收件箱视图
    Inbox,
    /// 今日任务视图
    Today,
    /// 计划任务视图
    Scheduled,
    /// 已完成任务视图
    Completed,
    /// 置顶任务视图
    Pinned,
    /// 项目视图
    Project(u64), // 使用 hash 作为 ID
    /// 标签视图
    Label(u64),
}

/// 变化类型
#[derive(Debug, Clone)]
pub enum ChangeType {
    /// 任务添加
    ItemAdded(Arc<ItemModel>),
    /// 任务更新
    ItemUpdated { old: Arc<ItemModel>, new: Arc<ItemModel> },
    /// 任务删除
    ItemDeleted(Arc<ItemModel>),
    /// 批量更新
    BulkUpdate,
}

impl ChangeType {
    /// 判断变化是否影响指定视图
    pub fn affects_view(&self, view_type: ViewType) -> bool {
        match self {
            ChangeType::ItemAdded(item) => Self::item_affects_view(item, view_type),
            ChangeType::ItemUpdated { old, new } => {
                // 如果旧项或新项影响视图，都需要更新
                Self::item_affects_view(old, view_type) || Self::item_affects_view(new, view_type)
            },
            ChangeType::ItemDeleted(item) => Self::item_affects_view(item, view_type),
            ChangeType::BulkUpdate => true, // 批量更新影响所有视图
        }
    }

    /// 判断任务是否影响指定视图
    fn item_affects_view(item: &ItemModel, view_type: ViewType) -> bool {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        match view_type {
            ViewType::Inbox => {
                // 收件箱：未完成且无项目
                !item.checked
                    && (item.project_id.is_none() || item.project_id.as_deref() == Some(""))
            },
            ViewType::Today => {
                // 今日任务：未完成且今日到期
                !item.checked && item.is_due_today()
            },
            ViewType::Scheduled => {
                // 计划任务：未完成且有截止日期
                !item.checked && item.due_date().is_some()
            },
            ViewType::Completed => {
                // 已完成任务
                item.checked
            },
            ViewType::Pinned => {
                // 置顶任务：未完成且已置顶
                !item.checked && item.pinned
            },
            ViewType::Project(project_hash) => {
                // 项目视图：属于指定项目
                if let Some(project_id) = &item.project_id {
                    let mut hasher = DefaultHasher::new();
                    project_id.hash(&mut hasher);
                    hasher.finish() == project_hash
                } else {
                    false
                }
            },
            ViewType::Label(_label_hash) => {
                // 标签视图：包含指定标签
                // 注意：ItemModel 的 labels 字段类型需要确认
                // 这里假设是 Vec<String> 或类似类型
                false // 暂时返回 false，需要根据实际的 labels 字段类型实现
            },
        }
    }
}

/// 观察者注册表
///
/// 管理视图的订阅关系，只通知受影响的视图
pub struct ObserverRegistry {
    /// 注册的观察者
    /// Key: 视图类型，Value: 观察者 ID 列表
    observers: std::collections::HashMap<ViewType, Vec<u64>>,
    /// 下一个观察者 ID
    next_id: u64,
}

impl Global for ObserverRegistry {}

impl ObserverRegistry {
    /// 创建新的观察者注册表
    pub fn new() -> Self {
        Self { observers: std::collections::HashMap::new(), next_id: 0 }
    }

    /// 注册观察者
    ///
    /// 返回观察者 ID，用于后续取消注册
    pub fn register(&mut self, view_type: ViewType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.observers.entry(view_type).or_default().push(id);

        id
    }

    /// 取消注册观察者
    pub fn unregister(&mut self, view_type: ViewType, id: u64) {
        if let Some(observers) = self.observers.get_mut(&view_type) {
            observers.retain(|&observer_id| observer_id != id);
        }
    }

    /// 获取受影响的视图类型
    pub fn get_affected_views(&self, change: &ChangeType) -> Vec<ViewType> {
        self.observers
            .keys()
            .filter(|&&view_type| change.affects_view(view_type))
            .copied()
            .collect()
    }

    /// 检查视图是否受影响
    pub fn is_view_affected(&self, view_type: ViewType, change: &ChangeType) -> bool {
        change.affects_view(view_type)
    }
}

impl Default for ObserverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 脏标记系统
///
/// 用于标记哪些视图需要更新
pub struct DirtyFlags {
    /// 脏视图集合
    dirty_views: std::collections::HashSet<ViewType>,
}

impl Global for DirtyFlags {}

impl DirtyFlags {
    /// 创建新的脏标记系统
    pub fn new() -> Self {
        Self { dirty_views: std::collections::HashSet::new() }
    }

    /// 标记视图为脏
    pub fn mark_dirty(&mut self, view_type: ViewType) {
        self.dirty_views.insert(view_type);
    }

    /// 检查视图是否为脏
    pub fn is_dirty(&self, view_type: ViewType) -> bool {
        self.dirty_views.contains(&view_type)
    }

    /// 清除视图的脏标记
    pub fn clear(&mut self, view_type: ViewType) {
        self.dirty_views.remove(&view_type);
    }

    /// 清除所有脏标记
    pub fn clear_all(&mut self) {
        self.dirty_views.clear();
    }

    /// 获取所有脏视图
    pub fn get_dirty_views(&self) -> Vec<ViewType> {
        self.dirty_views.iter().copied().collect()
    }
}

impl Default for DirtyFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// 选择性更新助手
///
/// 提供便捷的方法来判断是否需要更新视图
pub struct SelectiveUpdateHelper;

impl SelectiveUpdateHelper {
    /// 判断事件是否影响收件箱视图
    pub fn affects_inbox(event: &TodoStoreEvent) -> bool {
        match event {
            TodoStoreEvent::ItemAdded(_)
            | TodoStoreEvent::ItemUpdated(_)
            | TodoStoreEvent::ItemDeleted(_) => true,
            TodoStoreEvent::ProjectChanged(_) => true, // 项目变化可能影响收件箱
            TodoStoreEvent::BulkUpdate => true,
            TodoStoreEvent::ActiveProjectChanged => false,
        }
    }

    /// 判断事件是否影响今日任务视图
    pub fn affects_today(event: &TodoStoreEvent) -> bool {
        matches!(
            event,
            TodoStoreEvent::ItemAdded(_)
                | TodoStoreEvent::ItemUpdated(_)
                | TodoStoreEvent::ItemDeleted(_)
                | TodoStoreEvent::BulkUpdate
        )
    }

    /// 判断事件是否影响计划任务视图
    pub fn affects_scheduled(event: &TodoStoreEvent) -> bool {
        matches!(
            event,
            TodoStoreEvent::ItemAdded(_)
                | TodoStoreEvent::ItemUpdated(_)
                | TodoStoreEvent::ItemDeleted(_)
                | TodoStoreEvent::BulkUpdate
        )
    }

    /// 判断事件是否影响已完成任务视图
    pub fn affects_completed(event: &TodoStoreEvent) -> bool {
        matches!(
            event,
            TodoStoreEvent::ItemAdded(_)
                | TodoStoreEvent::ItemUpdated(_)
                | TodoStoreEvent::ItemDeleted(_)
                | TodoStoreEvent::BulkUpdate
        )
    }

    /// 判断事件是否影响项目视图
    pub fn affects_project(event: &TodoStoreEvent, _project_id: &str) -> bool {
        matches!(
            event,
            TodoStoreEvent::ItemAdded(_)
                | TodoStoreEvent::ItemUpdated(_)
                | TodoStoreEvent::ItemDeleted(_)
                | TodoStoreEvent::ProjectChanged(_)
                | TodoStoreEvent::BulkUpdate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_affects_inbox() {
        let item = Arc::new(ItemModel {
            id: "1".to_string(),
            content: "Test".to_string(),
            checked: false,
            project_id: None,
            ..Default::default()
        });

        let change = ChangeType::ItemAdded(item);
        assert!(change.affects_view(ViewType::Inbox));
    }

    #[test]
    fn test_change_not_affects_inbox() {
        let item = Arc::new(ItemModel {
            id: "1".to_string(),
            content: "Test".to_string(),
            checked: false,
            project_id: Some("project1".to_string()),
            ..Default::default()
        });

        let change = ChangeType::ItemAdded(item);
        assert!(!change.affects_view(ViewType::Inbox));
    }

    #[test]
    fn test_observer_registry() {
        let mut registry = ObserverRegistry::new();

        let id1 = registry.register(ViewType::Inbox);
        let id2 = registry.register(ViewType::Today);

        assert_eq!(registry.observers.len(), 2);

        registry.unregister(ViewType::Inbox, id1);
        assert_eq!(registry.observers.get(&ViewType::Inbox).unwrap().len(), 0);

        registry.unregister(ViewType::Today, id2);
    }

    #[test]
    fn test_dirty_flags() {
        let mut flags = DirtyFlags::new();

        flags.mark_dirty(ViewType::Inbox);
        assert!(flags.is_dirty(ViewType::Inbox));
        assert!(!flags.is_dirty(ViewType::Today));

        flags.clear(ViewType::Inbox);
        assert!(!flags.is_dirty(ViewType::Inbox));
    }
}
