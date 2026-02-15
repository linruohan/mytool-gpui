//! 统一的任务状态管理
//!
//! 这个模块提供了一个统一的 TodoStore，用于替代之前分散的多个状态结构。
//! 通过在内存中进行过滤，避免了多次数据库查询，提高了性能。

use std::sync::Arc;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use gpui::Global;
use serde_json::Value;
use todos::entity::{ItemModel, ProjectModel, SectionModel};

/// 统一的任务存储
///
/// 这是应用中所有任务数据的唯一数据源，各视图通过过滤方法获取所需数据。
pub struct TodoStore {
    /// 所有未完成的任务（唯一数据源）
    pub all_items: Vec<Arc<ItemModel>>,
    /// 所有项目
    pub projects: Vec<Arc<ProjectModel>>,
    /// 所有分区
    pub sections: Vec<Arc<SectionModel>>,
    /// 当前活跃项目
    pub active_project: Option<Arc<ProjectModel>>,
}

impl Global for TodoStore {}

impl TodoStore {
    /// 创建一个空的 TodoStore
    pub fn new() -> Self {
        Self { all_items: vec![], projects: vec![], sections: vec![], active_project: None }
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
        let today = Utc::now().date_naive();
        self.all_items
            .iter()
            .filter(|item| {
                if item.checked {
                    return false;
                }
                Self::is_due_today(&item.due, today)
            })
            .cloned()
            .collect()
    }

    /// 获取计划任务（有截止日期但未完成）
    pub fn scheduled_items(&self) -> Vec<Arc<ItemModel>> {
        self.all_items.iter().filter(|item| !item.checked && item.due.is_some()).cloned().collect()
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
        let now = Utc::now().naive_utc();
        self.all_items
            .iter()
            .filter(|item| {
                if item.checked {
                    return false;
                }
                Self::is_overdue(&item.due, now)
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
    }

    /// 更新所有项目
    pub fn set_projects(&mut self, projects: Vec<ProjectModel>) {
        self.projects = projects.into_iter().map(Arc::new).collect();
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
            self.all_items[pos] = item;
        } else {
            self.all_items.push(item);
        }
    }

    /// 删除单个任务
    pub fn remove_item(&mut self, id: &str) {
        self.all_items.retain(|i| i.id != id);
    }

    /// 添加单个任务
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.all_items.push(item);
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
            self.all_items.push(item);
        }

        // 处理更新
        for item in updated {
            if let Some(pos) = self.all_items.iter().position(|i| i.id == item.id) {
                self.all_items[pos] = item;
            }
        }

        // 处理删除
        for id in deleted {
            self.all_items.retain(|i| i.id != id);
        }
    }

    /// 从 JSON Value 中提取日期字符串
    fn extract_due_string(due: &Option<Value>) -> Option<String> {
        due.as_ref().and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else if let Some(obj) = v.as_object() {
                obj.get("date").and_then(|d| d.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
    }

    /// 检查任务是否在今天到期
    fn is_due_today(due: &Option<Value>, today: NaiveDate) -> bool {
        if let Some(due_str) = Self::extract_due_string(due) {
            if let Ok(due_datetime) = NaiveDateTime::parse_from_str(&due_str, "%Y-%m-%d %H:%M") {
                return due_datetime.date() == today;
            }
            if let Ok(due_date) = NaiveDate::parse_from_str(&due_str, "%Y-%m-%d") {
                return due_date == today;
            }
        }
        false
    }

    /// 检查任务是否已过期
    fn is_overdue(due: &Option<Value>, now: NaiveDateTime) -> bool {
        if let Some(due_str) = Self::extract_due_string(due) {
            if let Ok(due_datetime) = NaiveDateTime::parse_from_str(&due_str, "%Y-%m-%d %H:%M") {
                return due_datetime < now;
            }
        }
        false
    }
}

impl Default for TodoStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(id: &str, checked: bool, pinned: bool, due: Option<&str>) -> ItemModel {
        ItemModel {
            id: id.to_string(),
            checked,
            pinned,
            due: due.map(|d| serde_json::json!(d)),
            ..Default::default()
        }
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
