//! 统一的任务状态管理
//!
//! 这个模块提供了一个统一的 TodoStore，用于替代之前分散的多个状态结构。
//! 通过在内存中进行过滤，避免了多次数据库查询，提高了性能。
//!
//! ## 优化特性
//! - **增量索引更新**: 只更新变化的索引，避免全量重建
//! - **版本号机制**: 视图可以通过版本号判断是否需要更新
//! - **变更掩码**: 🚀 6.4优化，视图可按域筛选，避免无关回调执行
//! - **缓存集成**: 支持查询结果缓存，避免重复计算
//! - **索引操作抽象**: 通过 IndexOperation trait 统一索引操作逻辑

use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use gpui::Global;
use todos::entity::{ItemModel, LabelModel, ProjectModel, SectionModel};

// ==================== 变更掩码 ====================

/// 🚀 6.4优化：变更掩码，用于标记 TodoStore 中哪些数据发生了变化
///
/// 视图可以通过检查掩码来判断本次变更是否影响自己，
/// 避免不必要的列表重建和渲染。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangeMask {
    pub items_changed: bool,
    pub projects_changed: bool,
    pub sections_changed: bool,
    pub labels_changed: bool,
    pub active_project_changed: bool,
}

impl ChangeMask {
    /// 创建空掩码（无任何变更）
    pub const fn none() -> Self {
        Self {
            items_changed: false,
            projects_changed: false,
            sections_changed: false,
            labels_changed: false,
            active_project_changed: false,
        }
    }

    /// 创建全掩码（所有数据都变更）
    pub const fn all() -> Self {
        Self {
            items_changed: true,
            projects_changed: true,
            sections_changed: true,
            labels_changed: true,
            active_project_changed: true,
        }
    }

    /// 收件箱 / 今日 / 计划：任务列表或当前项目变化即需刷新
    #[inline]
    pub fn affects_item_filter_boards(&self) -> bool {
        self.items_changed || self.active_project_changed
    }

    pub fn affects_inbox(&self) -> bool {
        self.affects_item_filter_boards()
    }

    pub fn affects_today(&self) -> bool {
        self.affects_item_filter_boards()
    }

    pub fn affects_scheduled(&self) -> bool {
        self.affects_item_filter_boards()
    }

    /// 检查是否影响已完成视图
    pub fn affects_completed(&self) -> bool {
        self.items_changed
    }

    /// 检查是否影响置顶视图
    pub fn affects_pinned(&self) -> bool {
        self.items_changed
    }

    /// 检查是否影响项目视图
    pub fn affects_project(&self) -> bool {
        self.items_changed
            || self.projects_changed
            || self.sections_changed
            || self.active_project_changed
    }

    /// 检查是否影响标签看板（任务列表 + 标签集合）
    pub fn affects_labels(&self) -> bool {
        self.items_changed || self.labels_changed
    }

    /// 侧栏项目列表：仅项目集合变化时重建
    #[inline]
    pub fn affects_project_list(&self) -> bool {
        self.projects_changed
    }

    /// Section 管理列表：仅分区集合变化时重建
    #[inline]
    pub fn affects_section_list(&self) -> bool {
        self.sections_changed
    }

    /// 标签管理 / 标签弹出层：仅标签集合变化时重建
    #[inline]
    pub fn affects_label_list(&self) -> bool {
        self.labels_changed
    }

    /// 展开的任务编辑器：任务本身或下拉选项数据变化
    #[inline]
    pub fn affects_item_editor(&self) -> bool {
        self.items_changed || self.projects_changed || self.sections_changed || self.labels_changed
    }

    /// 合并两个掩码
    pub fn merge(&mut self, other: &Self) {
        self.items_changed |= other.items_changed;
        self.projects_changed |= other.projects_changed;
        self.sections_changed |= other.sections_changed;
        self.labels_changed |= other.labels_changed;
        self.active_project_changed |= other.active_project_changed;
    }

    /// 清空所有掩码位
    pub fn clear(&mut self) {
        *self = Self::none();
    }
}

// ==================== 看板成员分类 ====================

#[derive(Clone, Copy)]
struct BoardMembership {
    inbox: bool,
    today: bool,
    scheduled: bool,
}

fn board_membership(item: &ItemModel, today: chrono::NaiveDate) -> BoardMembership {
    if item.checked {
        return BoardMembership { inbox: false, today: false, scheduled: false };
    }
    let due = item.due_date_naive();
    let due_today = due == Some(today);
    let no_project = item.project_id.as_deref().is_none_or(|p| p.is_empty());
    BoardMembership { inbox: no_project && !due_today, today: due_today, scheduled: due.is_some() }
}

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

    /// 🚀 6.8优化：更新标签索引
    fn update_label_index(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 更新 id → item 映射
    fn update_id_map(&mut self, item: &Arc<ItemModel>, add: bool);

    /// 添加任务到所有索引
    fn add_to_all_indexes(&mut self, item: &Arc<ItemModel>) {
        self.update_id_map(item, true);
        self.update_project_index(item, true);
        self.update_section_index(item, true);
        self.update_checked_set(item, true);
        self.update_pinned_set(item, true);
        self.update_label_index(item, true);
    }

    /// 从所有索引移除任务
    fn remove_from_all_indexes(&mut self, item: &Arc<ItemModel>) {
        self.update_project_index(item, false);
        self.update_section_index(item, false);
        self.update_checked_set(item, false);
        self.update_pinned_set(item, false);
        self.update_label_index(item, false);
        self.update_id_map(item, false);
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
    /// 🚀 6.8优化：标签索引 - label_id -> item_ids 反查索引
    /// 避免每次查询时解析 JSON/字符串
    label_index: HashMap<String, Vec<String>>,
    /// id → item 映射，供索引反查 O(1) 取 Arc
    id_map: HashMap<String, Arc<ItemModel>>,
    /// 项目 / 分区 / 标签 O(1) 查找（与对应 Vec 同步维护）
    project_by_id: HashMap<String, Arc<ProjectModel>>,
    section_by_id: HashMap<String, Arc<SectionModel>>,
    label_by_id: HashMap<String, Arc<LabelModel>>,
    /// 看板成员索引：避免每次查询全表扫描并反复反序列化 due
    inbox_set: HashSet<String>,
    today_set: HashSet<String>,
    scheduled_set: HashSet<String>,
    /// 看板索引对应的本地日期（跨日时回退全表扫描）
    index_date: chrono::NaiveDate,

    /// 临时 ID 到真实 ID 的映射（用于 ID 变化检测）
    id_mappings: HashMap<String, String>,

    /// 版本号：每次数据变化时递增，用于优化观察者更新
    /// 视图可以通过比较版本号来判断是否需要重新渲染
    version: usize,

    /// 🚀 6.4优化：变更掩码，标记本次变更影响了哪些数据域
    /// 视图可通过检查掩码判断是否需要更新，避免惊群效应
    change_mask: ChangeMask,

    /// 上次 version 递增的时间戳，用于基于时间窗口的自动去重
    last_bump_time: Cell<Instant>,

    /// 任务列表 / 活跃项目变化代数。QueryCache 用它在 50ms 版本合并窗口内仍能失效。
    query_epoch: u64,

    /// 🚀 索引统计（用于性能监控）
    #[cfg(debug_assertions)]
    index_stats: IndexStats,
}

/// 索引统计信息
#[cfg(debug_assertions)]
#[derive(Debug, Default, Clone)]
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
            label_index: HashMap::new(),
            id_map: HashMap::new(),
            project_by_id: HashMap::new(),
            section_by_id: HashMap::new(),
            label_by_id: HashMap::new(),
            inbox_set: HashSet::new(),
            today_set: HashSet::new(),
            scheduled_set: HashSet::new(),
            index_date: chrono::Utc::now().naive_utc().date(),
            id_mappings: HashMap::new(),
            version: 0,
            change_mask: ChangeMask::none(),
            last_bump_time: Cell::new(Instant::now() - std::time::Duration::from_secs(1)),
            query_epoch: 0,
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

    /// 安全地递增版本号（基于时间窗口的防重入）
    ///
    /// 如果距上次 version++ 不足 50ms（同一事件循环/观察者分发窗口），
    /// 则跳过版本递增，掩码继续 OR 合并。真正递增时先清空掩码，
    /// 保证多 Board 可安全 peek 且掩码不会永久 sticky。
    #[inline]
    fn bump_version(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_bump_time.get());
        if elapsed.as_millis() >= 50 {
            self.version += 1;
            self.change_mask.clear();
            self.last_bump_time.set(now);
        }
    }

    fn mark_items_changed(&mut self) {
        self.bump_version();
        self.change_mask.items_changed = true;
        self.query_epoch = self.query_epoch.wrapping_add(1);
    }

    fn mark_projects_changed(&mut self) {
        self.bump_version();
        self.change_mask.projects_changed = true;
    }

    fn mark_sections_changed(&mut self) {
        self.bump_version();
        self.change_mask.sections_changed = true;
    }

    fn mark_labels_changed(&mut self) {
        self.bump_version();
        self.change_mask.labels_changed = true;
    }

    fn today_date() -> chrono::NaiveDate {
        chrono::Utc::now().naive_utc().date()
    }

    fn board_indexes_current(&self) -> bool {
        self.index_date == Self::today_date()
    }

    fn ensure_index_date(&mut self) {
        let today = Self::today_date();
        if self.index_date != today {
            self.rebuild_board_indexes();
        }
    }

    fn rebuild_board_indexes(&mut self) {
        self.inbox_set.clear();
        self.today_set.clear();
        self.scheduled_set.clear();
        self.index_date = Self::today_date();
        let items = self.all_items.clone();
        for item in items {
            self.apply_board_membership(&item, true);
        }
    }

    fn apply_board_membership(&mut self, item: &ItemModel, add: bool) {
        if !add {
            self.inbox_set.remove(&item.id);
            self.today_set.remove(&item.id);
            self.scheduled_set.remove(&item.id);
            return;
        }
        let m = board_membership(item, self.index_date);
        if m.inbox {
            self.inbox_set.insert(item.id.clone());
        } else {
            self.inbox_set.remove(&item.id);
        }
        if m.today {
            self.today_set.insert(item.id.clone());
        } else {
            self.today_set.remove(&item.id);
        }
        if m.scheduled {
            self.scheduled_set.insert(item.id.clone());
        } else {
            self.scheduled_set.remove(&item.id);
        }
    }

    fn items_from_set(&self, set: &HashSet<String>) -> Vec<Arc<ItemModel>> {
        if set.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(set.len());
        for item in &self.all_items {
            if set.contains(&item.id) {
                out.push(item.clone());
                if out.len() == set.len() {
                    break;
                }
            }
        }
        out
    }

    /// 查看当前变更掩码（不清空）
    pub fn peek_change_mask(&self) -> &ChangeMask {
        &self.change_mask
    }

    /// 获取临时 ID 对应的真实 ID
    pub fn get_real_id(&self, temp_id: &str) -> Option<&String> {
        self.id_mappings.get(temp_id)
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
        self.label_index.clear();
        self.id_map.clear();
        self.inbox_set.clear();
        self.today_set.clear();
        self.scheduled_set.clear();
        self.index_date = Self::today_date();

        // 用下标遍历，避免 clone 整表 Vec<Arc<_>>
        for i in 0..self.all_items.len() {
            let item = self.all_items[i].clone();
            self.add_to_all_indexes(&item);
            self.apply_board_membership(&item, true);
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

    /// 获取收件箱任务（未完成且无项目ID、且非今日到期）
    pub fn inbox_items(&self) -> Vec<Arc<ItemModel>> {
        if self.board_indexes_current() {
            return self.items_from_set(&self.inbox_set);
        }
        let today = Self::today_date();
        self.query_items(|item| {
            !item.checked
                && (item.project_id.is_none() || item.project_id.as_deref() == Some(""))
                && !item.is_due_on_date(today)
        })
    }

    fn cached_query(
        &self,
        cache: &crate::core::state::cache::QueryCache,
        kind: crate::core::state::cache::BoardQuery,
        compute: impl FnOnce() -> Vec<Arc<ItemModel>>,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        cache.get_or_compute(self.version, self.query_epoch, kind, compute)
    }

    /// 获取收件箱任务（带缓存）
    pub fn inbox_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        self.cached_query(cache, crate::core::state::cache::BoardQuery::Inbox, || {
            self.inbox_items()
        })
    }

    /// 获取今日到期的任务
    pub fn today_items(&self) -> Vec<Arc<ItemModel>> {
        if self.board_indexes_current() {
            return self.items_from_set(&self.today_set);
        }
        let today = Self::today_date();
        self.query_items(|item| !item.checked && item.is_due_on_date(today))
    }

    /// 获取今日到期的任务（带缓存）
    pub fn today_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        self.cached_query(cache, crate::core::state::cache::BoardQuery::Today, || {
            self.today_items()
        })
    }

    /// 获取计划任务（有截止日期但未完成）
    pub fn scheduled_items(&self) -> Vec<Arc<ItemModel>> {
        if self.board_indexes_current() {
            return self.items_from_set(&self.scheduled_set);
        }
        self.query_items(|item| !item.checked && item.due_date().is_some())
    }

    /// 获取计划任务（带缓存）
    pub fn scheduled_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        self.cached_query(cache, crate::core::state::cache::BoardQuery::Scheduled, || {
            self.scheduled_items()
        })
    }

    /// 获取已完成的任务（走 checked_set，保持 all_items 顺序）
    pub fn completed_items(&self) -> Vec<Arc<ItemModel>> {
        self.items_from_set(&self.checked_set)
    }

    /// 获取已完成的任务（带缓存）
    pub fn completed_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        self.cached_query(cache, crate::core::state::cache::BoardQuery::Completed, || {
            self.completed_items()
        })
    }

    /// 获取置顶任务（未完成且已置顶）
    pub fn pinned_items(&self) -> Vec<Arc<ItemModel>> {
        self.pinned_set
            .iter()
            .filter_map(|id| self.id_map.get(id))
            .filter(|item| !item.checked)
            .cloned()
            .collect()
    }

    /// 获取置顶任务（带缓存）
    pub fn pinned_items_cached(
        &self,
        cache: &crate::core::state::cache::QueryCache,
    ) -> Arc<Vec<Arc<ItemModel>>> {
        self.cached_query(cache, crate::core::state::cache::BoardQuery::Pinned, || {
            self.pinned_items()
        })
    }

    /// 获取指定项目的任务（走 project_index，不复制 Vec）
    pub fn items_by_project(&self, project_id: &str) -> &[Arc<ItemModel>] {
        self.project_index.get(project_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// 获取指定分区的任务（走 section_index，不复制 Vec）
    pub fn items_by_section(&self, section_id: &str) -> &[Arc<ItemModel>] {
        self.section_index.get(section_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// 获取指定标签的任务（label_index + id_map）
    pub fn items_by_label(&self, label_id: &str) -> Vec<Arc<ItemModel>> {
        if let Some(item_ids) = self.label_index.get(label_id) {
            item_ids.iter().filter_map(|id| self.id_map.get(id).cloned()).collect()
        } else {
            self.query_items(|item| {
                item.labels
                    .as_deref()
                    .map(|raw| raw.split(';').any(|id| id.trim() == label_id))
                    .unwrap_or(false)
            })
        }
    }

    /// 更新所有任务
    pub fn set_items(&mut self, items: Vec<ItemModel>) {
        self.all_items = items.into_iter().map(Arc::new).collect();
        self.rebuild_indexes();
        self.mark_items_changed();
    }

    fn rebuild_id_map<M>(
        items: &[Arc<M>],
        map: &mut HashMap<String, Arc<M>>,
        id_of: impl Fn(&M) -> &str,
    ) {
        map.clear();
        map.extend(items.iter().map(|item| (id_of(item).to_string(), item.clone())));
    }

    fn upsert_id_map<M>(
        items: &mut Vec<Arc<M>>,
        map: &mut HashMap<String, Arc<M>>,
        item: Arc<M>,
        id: &str,
        same_id: impl Fn(&M) -> bool,
    ) {
        if let Some(pos) = items.iter().position(|m| same_id(m)) {
            items[pos] = item.clone();
        } else {
            items.push(item.clone());
        }
        map.insert(id.to_string(), item);
    }

    fn insert_id_map<M>(
        items: &mut Vec<Arc<M>>,
        map: &mut HashMap<String, Arc<M>>,
        item: Arc<M>,
        id: &str,
    ) {
        map.insert(id.to_string(), item.clone());
        items.push(item);
    }

    /// 指定项目下的分区（过滤时不先 clone 整表）
    pub fn sections_for_project(&self, project_id: &str) -> Vec<Arc<SectionModel>> {
        self.sections
            .iter()
            .filter(|s| s.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect()
    }

    /// 更新所有项目
    pub fn set_projects(&mut self, projects: Vec<ProjectModel>) {
        self.projects = projects.into_iter().map(Arc::new).collect();
        Self::rebuild_id_map(&self.projects, &mut self.project_by_id, |p| &p.id);
        self.mark_projects_changed();
    }

    /// 更新所有标签
    pub fn set_labels(&mut self, labels: Vec<LabelModel>) {
        self.labels = labels.into_iter().map(Arc::new).collect();
        Self::rebuild_id_map(&self.labels, &mut self.label_by_id, |l| &l.id);
        self.mark_labels_changed();
    }

    /// 更新所有分区
    pub fn set_sections(&mut self, sections: Vec<SectionModel>) {
        self.sections = sections.into_iter().map(Arc::new).collect();
        Self::rebuild_id_map(&self.sections, &mut self.section_by_id, |s| &s.id);
        self.mark_sections_changed();
    }

    /// 设置活跃项目
    pub fn set_active_project(&mut self, project: Option<Arc<ProjectModel>>) {
        self.active_project = project;
        self.bump_version();
        self.change_mask.active_project_changed = true;
        self.query_epoch = self.query_epoch.wrapping_add(1);
    }

    // ==================== 增量更新方法 ====================

    /// 增量更新单个任务
    ///
    /// 如果任务已存在则更新，否则添加到列表末尾
    pub fn update_item(&mut self, item: Arc<ItemModel>) {
        self.upsert_item(item);
        self.mark_items_changed();
    }

    fn upsert_item(&mut self, item: Arc<ItemModel>) {
        self.ensure_index_date();
        tracing::debug!("TodoStore::update_item - id: {}, due: {:?}", item.id, item.due);

        if let Some(old_item) = self.id_map.get(&item.id).cloned() {
            if let Some(pos) = self.all_items.iter().position(|i| i.id == item.id) {
                self.all_items[pos] = item.clone();
            }
            self.update_item_index(&old_item, &item);
        } else {
            self.all_items.push(item.clone());
            self.add_item_to_index(&item);
        }
    }

    /// 删除单个任务
    pub fn remove_item(&mut self, id: &str) {
        self.remove_item_internal(id);
        self.mark_items_changed();
    }

    fn remove_item_internal(&mut self, id: &str) {
        self.ensure_index_date();
        if let Some(item) = self.id_map.get(id).cloned() {
            self.remove_item_from_index(&item);
            self.all_items.retain(|i| i.id != id);
        }
    }

    /// 原子地替换任务的 ID（用于临时 ID 变为真实 ID）
    pub fn replace_item_id(&mut self, old_id: &str, new_item: Arc<ItemModel>) {
        let new_id = new_item.id.clone();
        self.ensure_index_date();

        if let Some(old_item) = self.id_map.get(old_id).cloned() {
            self.remove_item_from_index(&old_item);
        }
        self.all_items.retain(|i| i.id != old_id);

        self.all_items.push(new_item.clone());
        self.add_item_to_index(&new_item);
        self.id_mappings.insert(old_id.to_string(), new_id);
        self.mark_items_changed();

        tracing::debug!("TodoStore: replaced temp ID {} with real ID {}", old_id, new_item.id);
    }

    /// 添加单个任务
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.add_item_internal(item);
        self.mark_items_changed();
    }

    fn add_item_internal(&mut self, item: Arc<ItemModel>) {
        self.ensure_index_date();
        self.all_items.push(item.clone());
        self.add_item_to_index(&item);
    }

    /// 根据ID获取单个任务
    pub fn get_item(&self, id: &str) -> Option<Arc<ItemModel>> {
        self.id_map.get(id).cloned()
    }

    /// 增量更新单个项目
    pub fn update_project(&mut self, project: Arc<ProjectModel>) {
        let id = project.id.clone();
        if self.active_project.as_ref().is_some_and(|p| p.id == id) {
            self.active_project = Some(project.clone());
        }
        Self::upsert_id_map(&mut self.projects, &mut self.project_by_id, project, &id, |p| {
            p.id == id
        });
        self.mark_projects_changed();
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
        self.project_by_id.remove(id);

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

        self.mark_projects_changed();

        next_project
    }

    /// 添加单个项目
    pub fn add_project(&mut self, project: Arc<ProjectModel>) {
        let id = project.id.clone();
        Self::insert_id_map(&mut self.projects, &mut self.project_by_id, project, &id);
        self.mark_projects_changed();
    }

    /// 将临时项目 ID 替换为落盘后的真实 ID
    pub fn replace_project_id(&mut self, old_id: &str, project: Arc<ProjectModel>) {
        if old_id == project.id {
            self.update_project(project);
            return;
        }
        let was_active = self.active_project.as_ref().is_some_and(|p| p.id == old_id);
        self.projects.retain(|p| p.id != old_id);
        self.project_by_id.remove(old_id);
        if was_active {
            self.active_project = Some(project.clone());
        }
        let id = project.id.clone();
        Self::insert_id_map(&mut self.projects, &mut self.project_by_id, project, &id);
        self.mark_projects_changed();
        if was_active {
            self.change_mask.active_project_changed = true;
        }
    }

    /// 根据ID获取单个项目
    pub fn get_project(&self, id: &str) -> Option<Arc<ProjectModel>> {
        self.project_by_id.get(id).cloned()
    }

    /// 增量更新单个分区
    pub fn update_section(&mut self, section: Arc<SectionModel>) {
        let id = section.id.clone();
        Self::upsert_id_map(&mut self.sections, &mut self.section_by_id, section, &id, |s| {
            s.id == id
        });
        self.mark_sections_changed();
    }

    /// 删除单个分区
    pub fn remove_section(&mut self, id: &str) {
        self.sections.retain(|s| s.id != id);
        self.section_by_id.remove(id);
        self.mark_sections_changed();
    }

    /// 添加单个分区
    pub fn add_section(&mut self, section: Arc<SectionModel>) {
        let id = section.id.clone();
        Self::insert_id_map(&mut self.sections, &mut self.section_by_id, section, &id);
        self.mark_sections_changed();
    }

    /// 根据ID获取单个分区
    pub fn get_section(&self, id: &str) -> Option<Arc<SectionModel>> {
        self.section_by_id.get(id).cloned()
    }

    // ==================== Label 增量更新方法 ====================

    /// 增量更新单个标签
    pub fn update_label(&mut self, label: Arc<LabelModel>) {
        let id = label.id.clone();
        Self::upsert_id_map(&mut self.labels, &mut self.label_by_id, label, &id, |l| l.id == id);
        self.mark_labels_changed();
    }

    /// 删除单个标签
    pub fn remove_label(&mut self, id: &str) {
        self.labels.retain(|l| l.id != id);
        self.label_by_id.remove(id);
        self.mark_labels_changed();
    }

    /// 添加单个标签
    pub fn add_label(&mut self, label: Arc<LabelModel>) {
        let id = label.id.clone();
        Self::insert_id_map(&mut self.labels, &mut self.label_by_id, label, &id);
        self.mark_labels_changed();
    }

    /// 根据ID获取单个标签
    pub fn get_label(&self, id: &str) -> Option<Arc<LabelModel>> {
        self.label_by_id.get(id).cloned()
    }

    // ==================== 索引管理辅助方法 ====================

    /// 将任务添加到索引（使用统一的 trait 方法）
    fn add_item_to_index(&mut self, item: &Arc<ItemModel>) {
        self.add_to_all_indexes(item);
        self.apply_board_membership(item, true);
    }

    fn remove_item_from_index(&mut self, item: &Arc<ItemModel>) {
        self.apply_board_membership(item, false);
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
            && let Some(items) = self.project_index.get_mut(project_id)
            && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
        {
            items[pos] = new_item.clone();
        }

        // 🚀 优化 2: 检查分区 ID 是否变化
        if old_item.section_id != new_item.section_id {
            self.update_section_index(old_item, false);
            self.update_section_index(new_item, true);
        } else if let Some(section_id) = &new_item.section_id
            && !section_id.is_empty()
            && let Some(items) = self.section_index.get_mut(section_id)
            && let Some(pos) = items.iter().position(|i| i.id == new_item.id)
        {
            items[pos] = new_item.clone();
        }

        // 始终刷新 id_map 中的 Arc
        self.update_id_map(new_item, true);

        // 🚀 优化 3: 检查完成状态是否变化
        if old_item.checked != new_item.checked {
            self.update_checked_set(new_item, true);
        }

        // 🚀 优化 4: 检查置顶状态是否变化
        if old_item.pinned != new_item.pinned {
            self.update_pinned_set(new_item, true);
        }

        // 🚀 6.8优化 5: 检查标签是否变化
        if old_item.labels != new_item.labels {
            self.update_label_index(old_item, false);
            self.update_label_index(new_item, true);
        }

        self.apply_board_membership(old_item, false);
        self.apply_board_membership(new_item, true);

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

    /// 🚀 6.8优化：更新标签索引
    ///
    /// 解析 item.labels（分号分隔的标签 ID 列表），
    /// 维护 label_id -> item_ids 的反查索引。
    fn update_label_index(&mut self, item: &Arc<ItemModel>, add: bool) {
        let Some(raw) = item.labels.as_deref() else {
            return;
        };
        if raw.is_empty() {
            return;
        }

        if add {
            // 添加：将 item.id 加入各标签对应的列表
            for label_id in raw.split(';') {
                let label_id = label_id.trim();
                if label_id.is_empty() {
                    continue;
                }
                let entry = self.label_index.entry(label_id.to_string()).or_default();
                if !entry.iter().any(|id| id == &item.id) {
                    entry.push(item.id.clone());
                }
            }
        } else {
            // 移除：从各标签对应的列表中移除 item.id
            for label_id in raw.split(';') {
                let label_id = label_id.trim();
                if label_id.is_empty() {
                    continue;
                }
                if let Some(items) = self.label_index.get_mut(label_id) {
                    items.retain(|id| id != &item.id);
                    if items.is_empty() {
                        self.label_index.remove(label_id);
                    }
                }
            }
        }
    }

    fn update_id_map(&mut self, item: &Arc<ItemModel>, add: bool) {
        if add {
            self.id_map.insert(item.id.clone(), item.clone());
        } else {
            self.id_map.remove(&item.id);
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

        store.set_items(vec![
            // 无项目、未完成、无日期 -> 应该在 Inbox
            create_test_item("1", false, false, None),
            // 无项目、已完成、无日期 -> 不应该在 Inbox
            create_test_item("2", true, false, None),
            // 无项目、未完成、有日期 -> 应该在 Inbox
            create_test_item("3", false, false, None),
            // 无项目、未完成、昨天日期 -> 应该在 Inbox (is_past_due = true)
            create_test_item("4", false, false, Some(&yesterday)),
            // 无项目、未完成、今天日期 -> 不应该在 Inbox (is_due_today = true)
            create_test_item("5", false, false, Some(&today)),
            // 无项目、未完成、明天日期 -> 应该在 Inbox (!is_due_today = true)
            create_test_item("6", false, false, Some(&tomorrow)),
            // 有项目、未完成 -> 不应该在 Inbox
            create_test_item_with_project("7", false, false, None, "proj1"),
        ]);

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
        store.set_items(vec![
            create_test_item("1", false, true, None),
            create_test_item("2", false, false, None),
            create_test_item("3", true, true, None),
        ]);

        let pinned = store.pinned_items();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].id, "1");
    }

    #[test]
    fn test_items_by_project_uses_index() {
        let mut store = TodoStore::new();
        store.set_items(vec![
            create_test_item_with_project("1", false, false, None, "p1"),
            create_test_item_with_project("2", false, false, None, "p1"),
            create_test_item_with_project("3", false, false, None, "p2"),
            create_test_item("4", false, false, None),
        ]);

        let p1 = store.items_by_project("p1");
        assert_eq!(p1.len(), 2);
        let p2 = store.items_by_project("p2");
        assert_eq!(p2.len(), 1);
    }

    #[test]
    fn test_label_index_rebuild_clears() {
        let mut store = TodoStore::new();
        let mut a = create_test_item("1", false, false, None);
        a.labels = Some("l1".to_string());
        store.set_items(vec![a.clone(), create_test_item("2", false, false, None)]);
        assert_eq!(store.items_by_label("l1").len(), 1);

        // 再次 set_items 应清空后重建，不应重复
        store.set_items(vec![a]);
        assert_eq!(store.items_by_label("l1").len(), 1);
    }

    #[test]
    fn test_incremental_board_indexes_follow_updates() {
        let mut store = TodoStore::new();
        let today = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        store.set_items(vec![
            create_test_item("inbox", false, false, None),
            create_test_item("today", false, false, Some(&today)),
        ]);
        assert_eq!(store.inbox_items().len(), 1);
        assert_eq!(store.today_items().len(), 1);
        assert_eq!(store.scheduled_items().len(), 1);

        let mut moved = create_test_item("inbox", false, false, Some(&today));
        moved.id = "inbox".to_string();
        store.update_item(Arc::new(moved));
        assert!(store.inbox_items().is_empty());
        assert_eq!(store.today_items().len(), 2);

        store.remove_item("today");
        assert_eq!(store.today_items().len(), 1);
        assert_eq!(store.completed_items().len(), 0);
    }

    #[test]
    fn test_cached_query_shares_arc() {
        let mut store = TodoStore::new();
        store.set_items(vec![create_test_item("1", false, false, None)]);
        let cache = crate::core::state::QueryCache::new();
        let a = store.inbox_items_cached(&cache);
        let b = store.inbox_items_cached(&cache);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.len(), 1);

        store.update_item(Arc::new(create_test_item("1", true, false, None)));
        let c = store.inbox_items_cached(&cache);
        assert!(!Arc::ptr_eq(&a, &c));
        assert!(c.is_empty());
    }

    #[test]
    fn test_entity_lookups_stay_indexed() {
        let mut store = TodoStore::new();

        let mut project = ProjectModel::default();
        project.id = "p1".to_string();
        project.name = "Alpha".to_string();
        store.set_projects(vec![project.clone()]);
        assert_eq!(store.get_project("p1").unwrap().name, "Alpha");

        project.name = "Beta".to_string();
        store.update_project(Arc::new(project));
        assert_eq!(store.get_project("p1").unwrap().name, "Beta");
        assert_eq!(store.projects.len(), 1);

        store.remove_project("p1");
        assert!(store.get_project("p1").is_none());
        assert!(store.projects.is_empty());

        let mut temp = ProjectModel::default();
        temp.id = "temp_p".to_string();
        temp.name = "Temp".to_string();
        store.add_project(Arc::new(temp));
        let mut real = ProjectModel::default();
        real.id = "real_p".to_string();
        real.name = "Temp".to_string();
        store.replace_project_id("temp_p", Arc::new(real));
        assert!(store.get_project("temp_p").is_none());
        assert_eq!(store.get_project("real_p").unwrap().name, "Temp");

        let mut section = SectionModel::default();
        section.id = "s1".to_string();
        section.project_id = Some("p2".to_string());
        store.add_section(Arc::new(section));
        assert!(store.get_section("s1").is_some());
        assert_eq!(store.sections_for_project("p2").len(), 1);
        store.remove_section("s1");
        assert!(store.get_section("s1").is_none());

        let mut label = LabelModel::default();
        label.id = "l1".to_string();
        store.add_label(Arc::new(label));
        assert!(store.get_label("l1").is_some());
        store.remove_label("l1");
        assert!(store.get_label("l1").is_none());
    }

    #[test]
    fn test_change_mask_list_vs_editor() {
        let mut items_only = ChangeMask::none();
        items_only.items_changed = true;
        assert!(items_only.affects_item_editor());
        assert!(!items_only.affects_project_list());
        assert!(!items_only.affects_label_list());
        assert!(!items_only.affects_section_list());

        let mut labels_only = ChangeMask::none();
        labels_only.labels_changed = true;
        assert!(labels_only.affects_label_list());
        assert!(labels_only.affects_item_editor());
        assert!(!labels_only.affects_project_list());
    }
}
