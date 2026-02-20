//! 缓存层 - 避免重复计算
//!
//! 这个模块提供了一个缓存层，用于缓存常用的查询结果，
//! 避免每次都重新过滤和计算。

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use gpui::Global;
use todos::entity::ItemModel;

/// 查询结果缓存
///
/// 缓存常用的查询结果，如收件箱任务、今日任务等
pub struct QueryCache {
    /// 收件箱任务缓存
    inbox_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 今日任务缓存
    today_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 计划任务缓存
    scheduled_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 已完成任务缓存
    completed_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 置顶任务缓存
    pinned_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 过期任务缓存
    overdue_cache: RefCell<Option<Vec<Arc<ItemModel>>>>,
    /// 项目任务缓存（按项目 ID）
    project_cache: RefCell<HashMap<String, Vec<Arc<ItemModel>>>>,
    /// 分区任务缓存（按分区 ID）
    section_cache: RefCell<HashMap<String, Vec<Arc<ItemModel>>>>,

    /// 缓存版本号（与 TodoStore 的版本号对应）
    cache_version: RefCell<usize>,
}

impl Global for QueryCache {}

impl QueryCache {
    /// 创建新的查询缓存
    pub fn new() -> Self {
        Self {
            inbox_cache: RefCell::new(None),
            today_cache: RefCell::new(None),
            scheduled_cache: RefCell::new(None),
            completed_cache: RefCell::new(None),
            pinned_cache: RefCell::new(None),
            overdue_cache: RefCell::new(None),
            project_cache: RefCell::new(HashMap::new()),
            section_cache: RefCell::new(HashMap::new()),
            cache_version: RefCell::new(0),
        }
    }

    /// 检查缓存是否有效
    pub fn is_valid(&self, store_version: usize) -> bool {
        *self.cache_version.borrow() == store_version
    }

    /// 更新缓存版本号
    pub fn update_version(&self, store_version: usize) {
        *self.cache_version.borrow_mut() = store_version;
    }

    /// 清空所有缓存
    pub fn invalidate_all(&self) {
        *self.inbox_cache.borrow_mut() = None;
        *self.today_cache.borrow_mut() = None;
        *self.scheduled_cache.borrow_mut() = None;
        *self.completed_cache.borrow_mut() = None;
        *self.pinned_cache.borrow_mut() = None;
        *self.overdue_cache.borrow_mut() = None;
        self.project_cache.borrow_mut().clear();
        self.section_cache.borrow_mut().clear();
    }

    /// 清空特定项目的缓存
    pub fn invalidate_project(&self, project_id: &str) {
        self.project_cache.borrow_mut().remove(project_id);
    }

    /// 清空特定分区的缓存
    pub fn invalidate_section(&self, section_id: &str) {
        self.section_cache.borrow_mut().remove(section_id);
    }

    // ==================== 收件箱缓存 ====================

    /// 获取收件箱缓存
    pub fn get_inbox(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.inbox_cache.borrow().clone()
    }

    /// 设置收件箱缓存
    pub fn set_inbox(&self, items: Vec<Arc<ItemModel>>) {
        *self.inbox_cache.borrow_mut() = Some(items);
    }

    // ==================== 今日任务缓存 ====================

    /// 获取今日任务缓存
    pub fn get_today(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.today_cache.borrow().clone()
    }

    /// 设置今日任务缓存
    pub fn set_today(&self, items: Vec<Arc<ItemModel>>) {
        *self.today_cache.borrow_mut() = Some(items);
    }

    // ==================== 计划任务缓存 ====================

    /// 获取计划任务缓存
    pub fn get_scheduled(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.scheduled_cache.borrow().clone()
    }

    /// 设置计划任务缓存
    pub fn set_scheduled(&self, items: Vec<Arc<ItemModel>>) {
        *self.scheduled_cache.borrow_mut() = Some(items);
    }

    // ==================== 已完成任务缓存 ====================

    /// 获取已完成任务缓存
    pub fn get_completed(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.completed_cache.borrow().clone()
    }

    /// 设置已完成任务缓存
    pub fn set_completed(&self, items: Vec<Arc<ItemModel>>) {
        *self.completed_cache.borrow_mut() = Some(items);
    }

    // ==================== 置顶任务缓存 ====================

    /// 获取置顶任务缓存
    pub fn get_pinned(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.pinned_cache.borrow().clone()
    }

    /// 设置置顶任务缓存
    pub fn set_pinned(&self, items: Vec<Arc<ItemModel>>) {
        *self.pinned_cache.borrow_mut() = Some(items);
    }

    // ==================== 过期任务缓存 ====================

    /// 获取过期任务缓存
    pub fn get_overdue(&self) -> Option<Vec<Arc<ItemModel>>> {
        self.overdue_cache.borrow().clone()
    }

    /// 设置过期任务缓存
    pub fn set_overdue(&self, items: Vec<Arc<ItemModel>>) {
        *self.overdue_cache.borrow_mut() = Some(items);
    }

    // ==================== 项目任务缓存 ====================

    /// 获取项目任务缓存
    pub fn get_project(&self, project_id: &str) -> Option<Vec<Arc<ItemModel>>> {
        self.project_cache.borrow().get(project_id).cloned()
    }

    /// 设置项目任务缓存
    pub fn set_project(&self, project_id: String, items: Vec<Arc<ItemModel>>) {
        self.project_cache.borrow_mut().insert(project_id, items);
    }

    // ==================== 分区任务缓存 ====================

    /// 获取分区任务缓存
    pub fn get_section(&self, section_id: &str) -> Option<Vec<Arc<ItemModel>>> {
        self.section_cache.borrow().get(section_id).cloned()
    }

    /// 设置分区任务缓存
    pub fn set_section(&self, section_id: String, items: Vec<Arc<ItemModel>>) {
        self.section_cache.borrow_mut().insert(section_id, items);
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_version() {
        let cache = QueryCache::new();
        assert!(cache.is_valid(0));

        cache.update_version(1);
        assert!(cache.is_valid(1));
        assert!(!cache.is_valid(0));
    }

    #[test]
    fn test_invalidate_all() {
        let cache = QueryCache::new();

        // 设置一些缓存
        cache.set_inbox(vec![]);
        cache.set_today(vec![]);

        // 清空缓存
        cache.invalidate_all();

        // 验证缓存已清空
        assert!(cache.get_inbox().is_none());
        assert!(cache.get_today().is_none());
    }
}
