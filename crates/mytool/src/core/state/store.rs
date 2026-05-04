//! 统一的任务状态管理
//!
//! 这个模块提供了一个统一的 TodoStore，用于替代之前分散的多个状态结构。
//! 通过在内存中进行过滤，避免了多次数据库查询，提高了性能。
//!
//! ## 优化特性
//! - **增量索引更新**: 只更新变化的索引，避免全量重建
//! - **版本号机制**: 视图可以通过版本号判断是否需要更新
//! - **缓存集成**: 支持查询结果缓存，避免重复计算
//! - **索引操作抽象**: 通过 IndexOperation trait 统一索引操作逻辑

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use gpui::Global;
use todos::entity::{ItemModel, LabelModel, ProjectModel, SectionModel};

// ==================== 索引操作 Trait ====================

/// 索引操作统一接口
///
/// 提供统一的索引更新方法，消除重复代码
trait IndexOperation {
    /// 更新项目索引
    ///
    /// # 参数
    /// - `item`: 要操作的任务
    /// - `add`: true 表示添加，false 表示移除
    fn update_project_index(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 更新分区索引
    fn update_section_index(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 更新完成状态索引
    fn update_checked_set(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 更新置顶状态索引
    fn update_pinned_set(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 添加任务到所有索引
    fn add_to_all_indexes(&mut self, item: &Arc<ItemModel>) {
        self.update_project_index(item, true);
        self.update_section_index(item, true);
        self.update_checked_set(item, true);
        self.update_pinned_set(item, true);
    }

    /// 从所有索引移除任务
    fn remove_from_all_indexes(&mut self, item: &Arc<ItemModel>) {
        self.update_project_index(item, false);
        self.update_section_index(item, false);
        self.update_checked_set(item, false);
        self.update_pinned_set(item, false);
    }
}

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

    /// 实际的索引重建实现（使用统一的 trait 方法）
    #[inline]
    fn rebuild_indexes_impl(&mut self) {
        self.project_index.clear();
        self.section_index.clear();
        self.checked_set.clear();
        self.pinned_set.clear();

        let items = self.all_items.clone();
        for item in &items {
            self.add_to_all_indexes(item);
        }
    }

    // ==================== 通用查询方法 ====================

    /// 通用查询方法
    ///
    /// 提供统一的查询接口，减少重复代码
    ///
    /// # 参数
    /// - `predicate`: 过滤条件的闭包
    ///
    /// # 示例
    /// ```ignore
    /// let items = store.query_items(|item| !item.checked && item.pinned);
    /// ```
    fn query_items(&self, predicate: impl Fn(&ItemModel) -> bool) -> Vec<Arc<ItemModel>> {
        self.all_items.iter().filter(|item| predicate(item)).cloned().collect()
    }

    /// 获取收件箱任务（未完成且无项目ID的任务）
    ///
    /// 使用通用查询方法
    pub fn inbox_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| {
            !item.checked
                && (item.project_id.is_none() || item.project_id.as_deref() == Some(""))
                && !item.is_due_today()
        })
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
    /// 使用通用查询方法
    pub fn today_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| !item.checked && item.is_due_today())
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
        self.query_items(|item| !item.checked && item.due_date().is_some())
    }

    /// 获取已完成的任务
    pub fn completed_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| item.checked)
    }

    /// 获取置顶任务（未完成且已置顶）
    pub fn pinned_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| !item.checked && item.pinned)
    }

    /// 获取过期任务
    pub fn overdue_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| !item.checked && item.is_overdue())
    }

    /// 获取指定项目的任务
    pub fn items_by_project(&self, project_id: &str) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| item.project_id.as_deref() == Some(project_id))
    }

    /// 获取指定项目的置顶任务（未完成且已置顶）
    pub fn pinned_items_by_project(&self, project_id: &str) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| {
            item.project_id.as_deref() == Some(project_id) && !item.checked && item.pinned
        })
    }

    /// 获取指定分区的任务
    pub fn items_by_section(&self, section_id: &str) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| item.section_id.as_deref() == Some(section_id))
    }

    /// 获取无分区的任务
    pub fn no_section_items(&self) -> Vec<Arc<ItemModel>> {
        self.query_items(|item| {
            !item.checked && (item.section_id.is_none() || item.section_id.as_deref() == Some(""))
        })
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
        tracing::info!("TodoStore::update_item called - id: {}, due: {:?}", item.id, item.due);

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

    /// 删除单个项目，并返回下一个应该激活的项目
    ///
    /// 删除逻辑：
    /// 1. 找到被删除项目的索引位置
    /// 2. 如果删除的是当前活跃项目，则自动选择下一个项目
    /// 3. 如果删除的是最后一个项目，则选择前一个项目
    /// 4. 如果没有其他项目了，返回 None
    pub fn remove_project(&mut self, id: &str) -> Option<Arc<ProjectModel>> {
        // 找到被删除项目的索引
        let removed_index = self.projects.iter().position(|p| p.id == id);

        // 从列表中移除项目
        self.projects.retain(|p| p.id != id);

        // 检查是否删除的是当前活跃项目
        let is_active_project = self.active_project.as_ref().map(|p| p.id == id).unwrap_or(false);

        // 计算下一个应该激活的项目
        let next_project = if is_active_project {
            if let Some(index) = removed_index {
                // 优先选择同一位置的下一个项目（因为删除后，原来的 index+1 变成了 index）
                // 如果 index 超出范围，则选择最后一个
                if index < self.projects.len() {
                    self.projects.get(index).cloned()
                } else if index > 0 {
                    self.projects.get(index - 1).cloned()
                } else {
                    // 如果只有一个项目且被删除了，返回 None
                    None
                }
            } else {
                None
            }
        } else {
            // 如果删除的不是当前活跃项目，保持当前活跃项目不变
            self.active_project.clone()
        };

        // 更新活跃项目
        if is_active_project {
            self.active_project = next_project.clone();
        }

        // 增加版本号
        self.version += 1;

        next_project
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

    /// 将任务添加到索引（使用统一的 trait 方法）
    fn add_item_to_index(&mut self, item: &Arc<ItemModel>) {
        self.add_to_all_indexes(item);
    }

    /// 从索引中移除任务（使用统一的 trait 方法）
    fn remove_item_from_index(&mut self, item: &Arc<ItemModel>) {
        self.remove_from_all_indexes(item);
    }

    /// 更新任务索引（处理状态变化）
    ///
    /// 🚀 性能优化：只更新变化的索引，而不是全部移除再添加
    fn update_item_index(&mut self, old_item: &Arc<ItemModel>, new_item: &Arc<ItemModel>) {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        // 🚀 优化 1: 检查项目 ID 是否变化
        if old_item.project_id != new_item.project_id {
            self.update_project_index(old_item, false);
            self.update_project_index(new_item, true);
        } else if let Some(project_id) = &new_item.project_id
            && !project_id.is_empty()
        {
            if let Some(items) = self.project_index.get_mut(project_id)
                && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
            {
                items[pos] = new_item.clone();
            }
        }

        // 🚀 优化 2: 检查分区 ID 是否变化
        if old_item.section_id != new_item.section_id {
            self.update_section_index(old_item, false);
            self.update_section_index(new_item, true);
        } else if let Some(section_id) = &new_item.section_id
            && !section_id.is_empty()
        {
            if let Some(items) = self.section_index.get_mut(section_id)
                && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
            {
                items[pos] = new_item.clone();
            }
        }

        // 🚀 优化 3: 检查完成状态是否变化
        if old_item.checked != new_item.checked {
            self.update_checked_set(new_item, true);
        }

        // 🚀 优化 4: 检查置顶状态是否变化
        if old_item.pinned != new_item.pinned {
            self.update_pinned_set(new_item, true);
        }

        #[cfg(debug_assertions)]
        {
            let duration = start.elapsed();
            self.index_stats.incremental_update_count += 1;

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

// ==================== IndexOperation Trait 实现 ====================

impl IndexOperation for TodoStore {
    /// 更新项目索引
    fn update_project_index(&mut self, item: &Arc<ItemModel>, add: bool) {
        if let Some(project_id) = &item.project_id
            && !project_id.is_empty()
        {
            if add {
                self.project_index.entry(project_id.clone()).or_default().push(item.clone());
            } else if let Some(items) = self.project_index.get_mut(project_id) {
                items.retain(|i| i.id != item.id);
                if items.is_empty() {
                    self.project_index.remove(project_id);
                }
            }
        }
    }

    /// 更新分区索引
    fn update_section_index(&mut self, item: &Arc<ItemModel>, add: bool) {
        if let Some(section_id) = &item.section_id
            && !section_id.is_empty()
        {
            if add {
                self.section_index.entry(section_id.clone()).or_default().push(item.clone());
            } else if let Some(items) = self.section_index.get_mut(section_id) {
                items.retain(|i| i.id != item.id);
                if items.is_empty() {
                    self.section_index.remove(section_id);
                }
            }
        }
    }

    /// 更新完成状态索引
    fn update_checked_set(&mut self, item: &Arc<ItemModel>, add: bool) {
        if add && item.checked {
            self.checked_set.insert(item.id.clone());
        } else {
            self.checked_set.remove(&item.id);
        }
    }

    /// 更新置顶状态索引
    fn update_pinned_set(&mut self, item: &Arc<ItemModel>, add: bool) {
        if add && item.pinned {
            self.pinned_set.insert(item.id.clone());
        } else {
            self.pinned_set.remove(&item.id);
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

    fn create_test_item_with_project(
        id: &str,
        checked: bool,
        pinned: bool,
        due: Option<&str>,
        project_id: &str,
    ) -> ItemModel {
        let mut item = create_test_item(id, checked, pinned, due);
        item.project_id = Some(project_id.to_string());
        item
    }

    #[test]
    fn test_inbox_items() {
        let mut store = TodoStore::new();

        // 创建测试数据
        let today = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let yesterday =
            (chrono::Utc::now() - chrono::Days::new(1)).format("%Y-%m-%d %H:%M:%S").to_string();
        let tomorrow =
            (chrono::Utc::now() + chrono::Days::new(1)).format("%Y-%m-%d %H:%M:%S").to_string();

        store.all_items = vec![
            // 无项目、未完成、无日期 -> 应该在 Inbox
            Arc::new(create_test_item("1", false, false, None)),
            // 无项目、已完成、无日期 -> 不应该在 Inbox
            Arc::new(create_test_item("2", true, false, None)),
            // 无项目、未完成、有日期 -> 应该在 Inbox
            Arc::new(create_test_item("3", false, false, None)),
            // 无项目、未完成、昨天日期 -> 应该在 Inbox (is_past_due = true)
            Arc::new(create_test_item("4", false, false, Some(&yesterday))),
            // 无项目、未完成、今天日期 -> 不应该在 Inbox (is_due_today = true)
            Arc::new(create_test_item("5", false, false, Some(&today))),
            // 无项目、未完成、明天日期 -> 应该在 Inbox (!is_due_today = true)
            Arc::new(create_test_item("6", false, false, Some(&tomorrow))),
            // 有项目、未完成 -> 不应该在 Inbox
            Arc::new(create_test_item_with_project("7", false, false, None, "proj1")),
        ];

        let inbox = store.inbox_items();
        // 应该在 Inbox: 1, 3, 4, 6 = 4 个
        assert_eq!(inbox.len(), 4);

        // 验证今天到期的任务不在 Inbox
        let ids: Vec<&str> = inbox.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"1"));
        assert!(ids.contains(&"3"));
        assert!(ids.contains(&"4"));
        assert!(ids.contains(&"6"));
        assert!(!ids.contains(&"2")); // 已完成
        assert!(!ids.contains(&"5")); // 今天到期
        assert!(!ids.contains(&"7")); // 有项目
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
