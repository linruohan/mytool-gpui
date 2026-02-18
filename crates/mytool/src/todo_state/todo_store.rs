//! 统一的任务状态管理
//!
//! 这个模块提供了一个统一的 TodoStore，用于替代之前分散的多个状态结构。
//! 通过在内存中进行过滤，避免了多次数据库查询，提高了性能。

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
        }
    }

    /// 重建所有索引
    /// 当批量更新数据时调用
    fn rebuild_indexes(&mut self) {
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

    /// 获取今日到期的任务
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
    }

    /// 更新所有项目
    pub fn set_projects(&mut self, projects: Vec<ProjectModel>) {
        self.projects = projects.into_iter().map(Arc::new).collect();
    }

    /// 更新所有标签
    pub fn set_labels(&mut self, labels: Vec<LabelModel>) {
        self.labels = labels.into_iter().map(Arc::new).collect();
    }

    /// 更新所有分区
    pub fn set_sections(&mut self, sections: Vec<SectionModel>) {
        self.sections = sections.into_iter().map(Arc::new).collect();
    }

    /// 设置活跃项目
    pub fn set_active_project(&mut self, project: Option<Arc<ProjectModel>>) {
        self.active_project = project;
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
    }

    /// 添加单个任务
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.all_items.push(item.clone());
        // 添加到索引
        self.add_item_to_index(&item);
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
    }

    /// 删除单个项目
    pub fn remove_project(&mut self, id: &str) {
        self.projects.retain(|p| p.id != id);
    }

    /// 添加单个项目
    pub fn add_project(&mut self, project: Arc<ProjectModel>) {
        self.projects.push(project);
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
    }

    /// 删除单个分区
    pub fn remove_section(&mut self, id: &str) {
        self.sections.retain(|s| s.id != id);
    }

    /// 添加单个分区
    pub fn add_section(&mut self, section: Arc<SectionModel>) {
        self.sections.push(section);
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
    }

    /// 删除单个标签
    pub fn remove_label(&mut self, id: &str) {
        self.labels.retain(|l| l.id != id);
    }

    /// 添加单个标签
    pub fn add_label(&mut self, label: Arc<LabelModel>) {
        self.labels.push(label);
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
        {
            if let Some(items) = self.project_index.get_mut(project_id) {
                items.retain(|i| i.id != item.id);
                // 如果该项目没有任务了，移除该条目
                if items.is_empty() {
                    self.project_index.remove(project_id);
                }
            }
        }

        // 分区索引
        if let Some(section_id) = &item.section_id
            && !section_id.is_empty()
        {
            if let Some(items) = self.section_index.get_mut(section_id) {
                items.retain(|i| i.id != item.id);
                // 如果该分区没有任务了，移除该条目
                if items.is_empty() {
                    self.section_index.remove(section_id);
                }
            }
        }

        // 检查状态索引
        self.checked_set.remove(&item.id);

        // 置顶状态索引
        self.pinned_set.remove(&item.id);
    }

    /// 更新任务索引（处理状态变化）
    fn update_item_index(&mut self, old_item: &Arc<ItemModel>, new_item: &Arc<ItemModel>) {
        // 先从索引中移除旧的
        self.remove_item_from_index(old_item);
        // 再添加新的
        self.add_item_to_index(new_item);
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
