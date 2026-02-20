//! 统一的任务状态管理
//!
//! 这个模块提供了一个统一的 TodoStore，用于替代之前分散的多个状态结构。
//! 通过在内存中进行过滤，避免了多次数据库查询，提高了性能。
//!
//! ## 优化特性
//! - **增量索引更新**: 只更新变化的索引，避免全量重建
//! - **版本号机制**: 视图可以通过版本号判断是否需要更新
//! - **缓存集成**: 支持查询结果缓存，避免重复计算

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::Global;
use todos::entity::{ItemModel, LabelModel, ProjectModel, SectionModel};

/// 统一的任务存储
///
/// 这是应用中所有数据的唯一数据源，各视图通过过滤方法获取所需数据。
pub struct TodoStore {
    /// 所有任务（唯一数据源）
    pub all_items: Vec<Arc<ItemModel>>,
    /// 所有项目
    pub projects: Vec<Arc<ProjectModel>>,
    /// 所有标签
    pub labels: Vec<Arc<LabelModel>>,
    /// 所有分区
    pub sections: Vec<Arc<SectionModel>>,
    /// 当前活跃项目
    pub active_project: Option<Arc<ProjectModel>>,

    /// 索引结构（用于优化查询性能）
    /// 项目索引：按 project_id 分组
    project_index: HashMap<String, Vec<Arc<ItemModel>>>,
    /// 分区索引：按 section_id 分组
    section_index: HashMap<String, Vec<Arc<ItemModel>>>,
    /// 检查状态索引：已完成的任务 ID
    checked_set: HashSet<String>,
    /// 置顶状态索引：已置顶的任务 ID
    pinned_set: HashSet<String>,

    /// 版本号：每次数据变化时递增，用于优化观察者更新
    /// 视图可以通过比较版本号来判断是否需要重新渲染
    version: usize,

    /// 🚀 索引统计（用于性能监控）
    #[cfg(debug_assertions)]
    index_stats: IndexStats,
}

/// 索引统计信息
#[cfg(debug_assertions)]
#[derive(Debug, Default)]
struct IndexStats {
    /// 索引重建次数
    rebuild_count: usize,
    /// 增量更新次数
    incremental_update_count: usize,
    /// 最后一次重建耗时（毫秒）
    last_rebuild_duration_ms: u128,
    /// 平均增量更新耗时（微秒）
    avg_incremental_update_us: u128,
}

impl Global for TodoStore {}

impl TodoStore {
    /// 创建一个空的 TodoStore
    pub fn new() -> Self {
        Self {
            all_items: vec![],
            projects: vec![],
            labels: vec![],
            sections: vec![],
            active_project: None,
            project_index: HashMap::new(),
            section_index: HashMap::new(),
            checked_set: HashSet::new(),
            pinned_set: HashSet::new(),
            version: 0,
            #[cfg(debug_assertions)]
            index_stats: IndexStats::default(),
        }
    }

    /// 获取当前版本号
    ///
    /// 视图可以缓存此版本号，在观察者回调中比较版本号来判断是否需要更新
    pub fn version(&self) -> usize {
        self.version
    }

    /// 🚀 获取索引统计信息（仅在 debug 模式下可用）
    #[cfg(debug_assertions)]
    pub fn index_stats(&self) -> &IndexStats {
        &self.index_stats
    }

    /// 🚀 打印索引统计信息（仅在 debug 模式下可用）
    #[cfg(debug_assertions)]
    pub fn print_index_stats(&self) {
        tracing::info!(
            "📊 Index Statistics:\n- Total items: {}\n- Rebuild count: {}\n- Incremental update \
             count: {}\n- Last rebuild duration: {}ms\n- Avg incremental update: {}μs\n- Project \
             index size: {}\n- Section index size: {}\n- Checked set size: {}\n- Pinned set size: \
             {}",
            self.all_items.len(),
            self.index_stats.rebuild_count,
            self.index_stats.incremental_update_count,
            self.index_stats.last_rebuild_duration_ms,
            self.index_stats.avg_incremental_update_us,
            self.project_index.len(),
            self.section_index.len(),
            self.checked_set.len(),
            self.pinned_set.len()
        );
    }

    /// 重建所有索引
    /// 当批量更新数据时调用
    ///
    /// ⚠️ 性能警告：这是一个 O(n) 操作，应该只在批量更新时使用
    /// 对于单个任务的增删改，请使用增量更新方法
    fn rebuild_indexes(&mut self) {
        #[cfg(debug_assertions)]
        {
            let start = std::time::Instant::now();
            tracing::debug!("Rebuilding all indexes for {} items", self.all_items.len());

            self.rebuild_indexes_impl();

            let duration = start.elapsed();
            self.index_stats.rebuild_count += 1;
            self.index_stats.last_rebuild_duration_ms = duration.as_millis();

            tracing::debug!(
                "Index rebuild #{} completed in {:?}",
                self.index_stats.rebuild_count,
                duration
            );

            if duration.as_millis() > 100 {
                tracing::warn!(
                    "Slow index rebuild detected: {:?} for {} items (rebuild #{})",
                    duration,
                    self.all_items.len(),
                    self.index_stats.rebuild_count
                );
            }
        }

        #[cfg(not(debug_assertions))]
        {
            self.rebuild_indexes_impl();
        }
    }

    /// 实际的索引重建实现
    #[inline]
    fn rebuild_indexes_impl(&mut self) {
        // 清空索引
        self.project_index.clear();
        self.section_index.clear();
        self.checked_set.clear();
        self.pinned_set.clear();

        // 重建索引
        for item in &self.all_items {
            // 项目索引
            if let Some(project_id) = &item.project_id
                && !project_id.is_empty()
            {
                self.project_index.entry(project_id.clone()).or_default().push(item.clone());
            }

            // 分区索引
            if let Some(section_id) = &item.section_id
                && !section_id.is_empty()
            {
                self.section_index.entry(section_id.clone()).or_default().push(item.clone());
            }

            // 检查状态索引
            if item.checked {
                self.checked_set.insert(item.id.clone());
            }

            // 置顶状态索引
            if item.pinned {
                self.pinned_set.insert(item.id.clone());
            }
        }
    }

    /// 获取收件箱任务（未完成且无项目ID的任务）
    ///
    /// 使用索引优化查询性能
    pub fn inbox_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| {
                !item.checked
                    && (item.project_id.is_none() || item.project_id.as_deref() == Some(""))
            })
            .cloned()
            .collect()
    }

    /// 获取收件箱任务（带缓存）
    ///
    /// 如果缓存有效，直接返回缓存结果；否则重新计算并更新缓存
    pub fn inbox_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Vec<Arc<ItemModel>> {
        // 检查缓存是否有效
        if cache.is_valid(self.version)
            && let Some(cached) = cache.get_inbox()
        {
            return cached;
        }

        // 缓存无效，重新计算
        let items = self.inbox_items();
        cache.set_inbox(items.clone());
        cache.update_version(self.version);
        items
    }

    /// 获取今日到期的任务
    ///
    /// 使用 ItemModel 的 is_due_today() 方法
    pub fn today_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| {
                if item.checked {
                    return false;
                }
                // 使用 ItemModel 的 is_due_today() 方法
                item.is_due_today()
            })
            .cloned()
            .collect()
    }

    /// 获取今日到期的任务（带缓存）
    pub fn today_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Vec<Arc<ItemModel>> {
        if cache.is_valid(self.version)
            && let Some(cached) = cache.get_today()
        {
            return cached;
        }

        let items = self.today_items();
        cache.set_today(items.clone());
        cache.update_version(self.version);
        items
    }

    /// 获取计划任务（有截止日期但未完成）
    pub fn scheduled_items(&self) -> Vec<Arc<ItemModel>> {
        // 使用 ItemModel 的 due_date() 方法检查是否有截止日期
        self.all_items
            .iter()
            .filter(|item| !item.checked && item.due_date().is_some())
            .cloned()
            .collect()
    }

    /// 获取已完成的任务
    pub fn completed_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items.iter().filter(|item| item.checked).cloned().collect()
    }

    /// 获取置顶任务（未完成且已置顶）
    pub fn pinned_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items.iter().filter(|item| !item.checked && item.pinned).cloned().collect()
    }

    /// 获取过期任务
    pub fn overdue_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| {
                if item.checked {
                    return false;
                }
                // 使用 ItemModel 的 is_overdue() 方法
                item.is_overdue()
            })
            .cloned()
            .collect()
    }

    /// 获取指定项目的任务
    pub fn items_by_project(&self, project_id: &str) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| item.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect()
    }

    /// 获取指定分区的任务
    pub fn items_by_section(&self, section_id: &str) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| item.section_id.as_deref() == Some(section_id))
            .cloned()
            .collect()
    }

    /// 获取无分区的任务
    pub fn no_section_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items
            .iter()
            .filter(|item| {
                !item.checked
                    && (item.section_id.is_none() || item.section_id.as_deref() == Some(""))
            })
            .cloned()
            .collect()
    }

    /// 更新所有任务
    pub fn set_items(&mut self, items: Vec<ItemModel>) {
        self.all_items = items.into_iter().map(Arc::new).collect();
        // 重建索引
        self.rebuild_indexes();
        // 增加版本号
        self.version += 1;
    }

    /// 更新所有项目
    pub fn set_projects(&mut self, projects: Vec<ProjectModel>) {
        self.projects = projects.into_iter().map(Arc::new).collect();
        // 增加版本号
        self.version += 1;
    }

    /// 更新所有标签
    pub fn set_labels(&mut self, labels: Vec<LabelModel>) {
        self.labels = labels.into_iter().map(Arc::new).collect();
        // 增加版本号
        self.version += 1;
    }

    /// 更新所有分区
    pub fn set_sections(&mut self, sections: Vec<SectionModel>) {
        self.sections = sections.into_iter().map(Arc::new).collect();
        // 增加版本号
        self.version += 1;
    }

    /// 设置活跃项目
    pub fn set_active_project(&mut self, project: Option<Arc<ProjectModel>>) {
        self.active_project = project;
        // 增加版本号
        self.version += 1;
    }

    // ==================== 增量更新方法 ====================

    /// 增量更新单个任务
    ///
    /// 如果任务已存在则更新，否则添加到列表末尾
    pub fn update_item(&mut self, item: Arc<ItemModel>) {
        if let Some(pos) = self.all_items.iter().position(|i| i.id == item.id) {
            // 先克隆 old_item，避免借用冲突
            let old_item = self.all_items[pos].clone();
            // 更新现有任务
            self.all_items[pos] = item.clone();

            // 更新索引
            self.update_item_index(&old_item, &item);
        } else {
            // 添加新任务
            self.all_items.push(item.clone());

            // 添加到索引
            self.add_item_to_index(&item);
        }
        // 增加版本号
        self.version += 1;
    }

    /// 删除单个任务
    pub fn remove_item(&mut self, id: &str) {
        // 先找到要删除的任务并克隆
        let item_to_remove = self.all_items.iter().find(|i| i.id == id).cloned();

        // 从索引中移除
        if let Some(item) = item_to_remove {
            self.remove_item_from_index(&item);
        }

        // 从列表中移除
        self.all_items.retain(|i| i.id != id);
        // 增加版本号
        self.version += 1;
    }

    /// 添加单个任务
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.all_items.push(item.clone());
        // 添加到索引
        self.add_item_to_index(&item);
        // 增加版本号
        self.version += 1;
    }

    /// 根据ID获取单个任务
    pub fn get_item(&self, id: &str) -> Option<Arc<ItemModel>> {
        self.all_items.iter().find(|i| i.id == id).cloned()
    }

    /// 增量更新单个项目
    pub fn update_project(&mut self, project: Arc<ProjectModel>) {
        if let Some(pos) = self.projects.iter().position(|p| p.id == project.id) {
            self.projects[pos] = project;
        } else {
            self.projects.push(project);
        }
        // 增加版本号
        self.version += 1;
    }

    /// 删除单个项目
    pub fn remove_project(&mut self, id: &str) {
        self.projects.retain(|p| p.id != id);
        // 增加版本号
        self.version += 1;
    }

    /// 添加单个项目
    pub fn add_project(&mut self, project: Arc<ProjectModel>) {
        self.projects.push(project);
        // 增加版本号
        self.version += 1;
    }

    /// 根据ID获取单个项目
    pub fn get_project(&self, id: &str) -> Option<Arc<ProjectModel>> {
        self.projects.iter().find(|p| p.id == id).cloned()
    }

    /// 增量更新单个分区
    pub fn update_section(&mut self, section: Arc<SectionModel>) {
        if let Some(pos) = self.sections.iter().position(|s| s.id == section.id) {
            self.sections[pos] = section;
        } else {
            self.sections.push(section);
        }
        // 增加版本号
        self.version += 1;
    }

    /// 删除单个分区
    pub fn remove_section(&mut self, id: &str) {
        self.sections.retain(|s| s.id != id);
        // 增加版本号
        self.version += 1;
    }

    /// 添加单个分区
    pub fn add_section(&mut self, section: Arc<SectionModel>) {
        self.sections.push(section);
        // 增加版本号
        self.version += 1;
    }

    /// 根据ID获取单个分区
    pub fn get_section(&self, id: &str) -> Option<Arc<SectionModel>> {
        self.sections.iter().find(|s| s.id == id).cloned()
    }

    // ==================== Label 增量更新方法 ====================

    /// 增量更新单个标签
    pub fn update_label(&mut self, label: Arc<LabelModel>) {
        if let Some(pos) = self.labels.iter().position(|l| l.id == label.id) {
            self.labels[pos] = label;
        } else {
            self.labels.push(label);
        }
        // 增加版本号
        self.version += 1;
    }

    /// 删除单个标签
    pub fn remove_label(&mut self, id: &str) {
        self.labels.retain(|l| l.id != id);
        // 增加版本号
        self.version += 1;
    }

    /// 添加单个标签
    pub fn add_label(&mut self, label: Arc<LabelModel>) {
        self.labels.push(label);
        // 增加版本号
        self.version += 1;
    }

    /// 根据ID获取单个标签
    pub fn get_label(&self, id: &str) -> Option<Arc<LabelModel>> {
        self.labels.iter().find(|l| l.id == id).cloned()
    }

    /// 批量增量更新
    ///
    /// 用于批量操作，如导入数据
    pub fn apply_changes(
        &mut self,
        added: Vec<Arc<ItemModel>>,
        updated: Vec<Arc<ItemModel>>,
        deleted: Vec<String>,
    ) {
        // 处理新增
        for item in added {
            self.add_item(item);
        }

        // 处理更新
        for item in updated {
            self.update_item(item);
        }

        // 处理删除
        for id in deleted {
            self.remove_item(&id);
        }
    }

    // ==================== 索引管理辅助方法 ====================

    /// 将任务添加到索引
    fn add_item_to_index(&mut self, item: &Arc<ItemModel>) {
        // 项目索引
        if let Some(project_id) = &item.project_id
            && !project_id.is_empty()
        {
            self.project_index.entry(project_id.clone()).or_default().push(item.clone());
        }

        // 分区索引
        if let Some(section_id) = &item.section_id
            && !section_id.is_empty()
        {
            self.section_index.entry(section_id.clone()).or_default().push(item.clone());
        }

        // 检查状态索引
        if item.checked {
            self.checked_set.insert(item.id.clone());
        }

        // 置顶状态索引
        if item.pinned {
            self.pinned_set.insert(item.id.clone());
        }
    }

    /// 从索引中移除任务
    fn remove_item_from_index(&mut self, item: &Arc<ItemModel>) {
        // 项目索引
        if let Some(project_id) = &item.project_id
            && !project_id.is_empty()
            && let Some(items) = self.project_index.get_mut(project_id)
        {
            items.retain(|i| i.id != item.id);
            // 如果该项目没有任务了，移除该条目
            if items.is_empty() {
                self.project_index.remove(project_id);
            }
        }

        // 分区索引
        if let Some(section_id) = &item.section_id
            && !section_id.is_empty()
            && let Some(items) = self.section_index.get_mut(section_id)
        {
            items.retain(|i| i.id != item.id);
            // 如果该分区没有任务了，移除该条目
            if items.is_empty() {
                self.section_index.remove(section_id);
            }
        }

        // 检查状态索引
        self.checked_set.remove(&item.id);

        // 置顶状态索引
        self.pinned_set.remove(&item.id);
    }

    /// 更新任务索引（处理状态变化）
    ///
    /// 🚀 性能优化：只更新变化的索引，而不是全部移除再添加
    fn update_item_index(&mut self, old_item: &Arc<ItemModel>, new_item: &Arc<ItemModel>) {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        // 🚀 优化 1: 检查项目 ID 是否变化
        if old_item.project_id != new_item.project_id {
            // 从旧项目索引移除
            if let Some(old_project_id) = &old_item.project_id
                && !old_project_id.is_empty()
                && let Some(items) = self.project_index.get_mut(old_project_id)
            {
                items.retain(|i| i.id != old_item.id);
                if items.is_empty() {
                    self.project_index.remove(old_project_id);
                }
            }

            // 添加到新项目索引
            if let Some(new_project_id) = &new_item.project_id
                && !new_project_id.is_empty()
            {
                self.project_index
                    .entry(new_project_id.clone())
                    .or_default()
                    .push(new_item.clone());
            }
        } else if let Some(project_id) = &new_item.project_id
            && !project_id.is_empty()
        {
            // 项目 ID 未变化，但需要更新引用
            if let Some(items) = self.project_index.get_mut(project_id)
                && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
            {
                items[pos] = new_item.clone();
            }
        }

        // 🚀 优化 2: 检查分区 ID 是否变化
        if old_item.section_id != new_item.section_id {
            // 从旧分区索引移除
            if let Some(old_section_id) = &old_item.section_id
                && !old_section_id.is_empty()
                && let Some(items) = self.section_index.get_mut(old_section_id)
            {
                items.retain(|i| i.id != old_item.id);
                if items.is_empty() {
                    self.section_index.remove(old_section_id);
                }
            }

            // 添加到新分区索引
            if let Some(new_section_id) = &new_item.section_id
                && !new_section_id.is_empty()
            {
                self.section_index
                    .entry(new_section_id.clone())
                    .or_default()
                    .push(new_item.clone());
            }
        } else if let Some(section_id) = &new_item.section_id
            && !section_id.is_empty()
        {
            // 分区 ID 未变化，但需要更新引用
            if let Some(items) = self.section_index.get_mut(section_id)
                && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
            {
                items[pos] = new_item.clone();
            }
        }

        // 🚀 优化 3: 检查完成状态是否变化
        if old_item.checked != new_item.checked {
            if new_item.checked {
                self.checked_set.insert(new_item.id.clone());
            } else {
                self.checked_set.remove(&new_item.id);
            }
        }

        // 🚀 优化 4: 检查置顶状态是否变化
        if old_item.pinned != new_item.pinned {
            if new_item.pinned {
                self.pinned_set.insert(new_item.id.clone());
            } else {
                self.pinned_set.remove(&new_item.id);
            }
        }

        #[cfg(debug_assertions)]
        {
            let duration = start.elapsed();
            self.index_stats.incremental_update_count += 1;

            // 计算移动平均
            let count = self.index_stats.incremental_update_count as u128;
            let old_avg = self.index_stats.avg_incremental_update_us;
            let new_duration_us = duration.as_micros();
            self.index_stats.avg_incremental_update_us =
                (old_avg * (count - 1) + new_duration_us) / count;

            if duration.as_micros() > 1000 {
                tracing::warn!(
                    "Slow incremental index update: {:?} (update #{})",
                    duration,
                    self.index_stats.incremental_update_count
                );
            }
        }
    }
}

impl Default for TodoStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use todos::DueDate;

    use super::*;

    fn create_test_item(id: &str, checked: bool, pinned: bool, due: Option<&str>) -> ItemModel {
        let due_json = due.map(|d| {
            // 创建完整的 DueDate 结构
            let due_date = DueDate {
                date: d.to_string(),
                timezone: "UTC".to_string(),
                recurrency_weeks: "".to_string(),
                is_recurring: false,
                recurrency_type: todos::enums::RecurrencyType::NONE,
                recurrency_interval: 0,
                recurrency_count: 0,
                recurrency_end: "".to_string(),
                recurrency_supported: false,
            };
            serde_json::to_value(due_date).unwrap()
        });

        ItemModel { id: id.to_string(), checked, pinned, due: due_json, ..Default::default() }
    }

    #[test]
    fn test_inbox_items() {
        let mut store = TodoStore::new();
        store.all_items = vec![
            Arc::new(create_test_item("1", false, false, None)),
            Arc::new(create_test_item("2", true, false, None)),
            Arc::new(create_test_item("3", false, false, None)),
        ];

        let inbox = store.inbox_items();
        assert_eq!(inbox.len(), 2);
    }

    #[test]
    fn test_pinned_items() {
        let mut store = TodoStore::new();
        store.all_items = vec![
            Arc::new(create_test_item("1", false, true, None)),
            Arc::new(create_test_item("2", false, false, None)),
            Arc::new(create_test_item("3", true, true, None)),
        ];

        let pinned = store.pinned_items();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].id, "1");
    }
}
