//! 缓存层 - 避免重复计算
//!
//! 看板查询结果按 `BoardQuery` 分槽缓存，命中时只克隆 Arc。

use std::{cell::RefCell, sync::Arc};

use gpui::Global;
use todos::entity::ItemModel;

type ItemList = Arc<Vec<Arc<ItemModel>>>;

const SLOT_COUNT: usize = 5;

/// 看板查询槽位（与 `QueryCache` 数组下标对应）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum BoardQuery {
    Inbox = 0,
    Today = 1,
    Scheduled = 2,
    Completed = 3,
    Pinned = 4,
}

/// 查询结果缓存
pub struct QueryCache {
    slots: [RefCell<Option<ItemList>>; SLOT_COUNT],
    cache_version: RefCell<usize>,
    query_epoch: RefCell<u64>,
}

impl Global for QueryCache {}

impl QueryCache {
    pub fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| RefCell::new(None)),
            cache_version: RefCell::new(0),
            query_epoch: RefCell::new(0),
        }
    }

    pub fn is_valid(&self, store_version: usize) -> bool {
        *self.cache_version.borrow() == store_version
    }

    pub fn update_version(&self, store_version: usize) {
        *self.cache_version.borrow_mut() = store_version;
    }

    pub fn invalidate_all(&self) {
        for slot in &self.slots {
            *slot.borrow_mut() = None;
        }
    }

    fn slot(&self, kind: BoardQuery) -> &RefCell<Option<ItemList>> {
        &self.slots[kind as usize]
    }

    pub fn get(&self, kind: BoardQuery) -> Option<ItemList> {
        self.slot(kind).borrow().clone()
    }

    pub fn set(&self, kind: BoardQuery, items: ItemList) {
        *self.slot(kind).borrow_mut() = Some(items);
    }

    /// 版本或任务代数变化时刷新槽位；仅 version 变（如改标签）则保留任务列表缓存。
    pub fn get_or_compute(
        &self,
        store_version: usize,
        query_epoch: u64,
        kind: BoardQuery,
        compute: impl FnOnce() -> Vec<Arc<ItemModel>>,
    ) -> ItemList {
        if *self.query_epoch.borrow() != query_epoch {
            self.invalidate_all();
            *self.query_epoch.borrow_mut() = query_epoch;
        }
        if *self.cache_version.borrow() != store_version {
            *self.cache_version.borrow_mut() = store_version;
        }

        if let Some(cached) = self.get(kind) {
            return cached;
        }

        let items = Arc::new(compute());
        self.set(kind, items.clone());
        items
    }

    pub fn get_inbox(&self) -> Option<ItemList> {
        self.get(BoardQuery::Inbox)
    }

    pub fn set_inbox(&self, items: ItemList) {
        self.set(BoardQuery::Inbox, items);
    }

    pub fn get_today(&self) -> Option<ItemList> {
        self.get(BoardQuery::Today)
    }

    pub fn get_scheduled(&self) -> Option<ItemList> {
        self.get(BoardQuery::Scheduled)
    }

    pub fn get_completed(&self) -> Option<ItemList> {
        self.get(BoardQuery::Completed)
    }

    pub fn get_pinned(&self) -> Option<ItemList> {
        self.get(BoardQuery::Pinned)
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

        cache.set_inbox(Arc::new(vec![]));
        cache.set(BoardQuery::Today, Arc::new(vec![]));

        cache.invalidate_all();

        assert!(cache.get_inbox().is_none());
        assert!(cache.get_today().is_none());
    }

    #[test]
    fn test_arc_hit_shares_allocation() {
        let cache = QueryCache::new();
        let items = Arc::new(vec![]);
        cache.set_inbox(items.clone());

        let a = cache.get_inbox().unwrap();
        let b = cache.get_inbox().unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(Arc::ptr_eq(&a, &items));
    }

    #[test]
    fn test_get_or_compute_fills_one_slot() {
        let cache = QueryCache::new();
        let a = cache.get_or_compute(1, 1, BoardQuery::Inbox, || vec![]);
        let b = cache.get_or_compute(1, 1, BoardQuery::Inbox, || panic!("should hit"));
        assert!(Arc::ptr_eq(&a, &b));
        assert!(cache.get_today().is_none());
    }

    #[test]
    fn test_label_only_change_keeps_item_slots() {
        let cache = QueryCache::new();
        cache.get_or_compute(1, 1, BoardQuery::Inbox, || vec![]);
        let kept = cache.get_or_compute(2, 1, BoardQuery::Inbox, || panic!("should keep"));
        assert!(kept.is_empty());
    }

    #[test]
    fn test_same_version_new_epoch_invalidates() {
        let cache = QueryCache::new();
        let first = cache.get_or_compute(1, 1, BoardQuery::Inbox, || vec![]);
        let second = cache.get_or_compute(1, 2, BoardQuery::Inbox, || vec![]);
        assert!(!Arc::ptr_eq(&first, &second));
    }
}
