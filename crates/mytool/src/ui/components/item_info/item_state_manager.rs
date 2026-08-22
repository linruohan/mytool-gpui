use std::sync::Arc;

use todos::entity::ItemModel;

/// 集中的状态管理结构
/// 用于统一管理 item 的状态更新，减少手动同步
pub struct ItemStateManager {
    /// 任务模型
    pub item: Arc<ItemModel>,
    /// 原始任务模型（用于取消时恢复）
    original_item: Arc<ItemModel>,
    /// 避免重复更新的标志
    pub skip_next_update: bool,
    /// 标记是否有未保存的修改
    has_unsaved_changes: bool,
    /// 是否是新建任务
    is_new_item: bool,
    /// 🚀 7.0新增：保存状态（用于 UI 显示）
    pub save_status: SaveItemStatus,
}

/// 🚀 7.0新增：保存状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveItemStatus {
    /// 空闲（无正在进行的保存操作）
    Idle,
    /// 正在保存
    Saving,
    /// 保存成功
    Succeeded,
    /// 保存失败
    Failed,
}

impl ItemStateManager {
    /// 创建新的 ItemStateManager
    pub fn new(item: Arc<ItemModel>) -> Self {
        let is_new = item.id.is_empty() || item.id.starts_with("temp_");
        Self {
            original_item: item.clone(),
            item,
            skip_next_update: false,
            has_unsaved_changes: false,
            is_new_item: is_new,
            save_status: SaveItemStatus::Idle,
        }
    }

    /// 检查是否是新建任务
    pub fn is_new_item(&self) -> bool {
        self.is_new_item
    }

    /// 恢复到原始数据（取消编辑）
    pub fn revert_to_original(&mut self) {
        self.item = self.original_item.clone();
        self.has_unsaved_changes = false;
    }

    /// 更新原始数据（保存成功后调用）
    pub fn update_original(&mut self) {
        self.original_item = self.item.clone();
        self.has_unsaved_changes = false;
        self.is_new_item = false;
    }

    /// 标记有未保存的修改
    pub fn mark_dirty(&mut self) {
        self.has_unsaved_changes = true;
    }

    /// 标记所有修改已保存
    pub fn mark_clean(&mut self) {
        self.has_unsaved_changes = false;
    }

    /// 检查是否有未保存的修改
    pub fn is_dirty(&self) -> bool {
        self.has_unsaved_changes
    }

    /// 统一的状态更新方法
    /// 使用闭包来修改 item 数据
    ///
    /// 性能注意：每次调用都会克隆整个 ItemModel
    /// 考虑批量更新以减少克隆次数
    pub fn update_item<F>(&mut self, f: F)
    where
        F: FnOnce(&mut ItemModel),
    {
        let mut item_data = (*self.item).clone();
        f(&mut item_data);
        self.item = Arc::new(item_data);
        self.has_unsaved_changes = true;
    }

    /// 设置项目 ID
    pub fn set_project_id(&mut self, project_id: Option<String>) {
        self.update_item(|item| {
            item.project_id = project_id;
        });
    }

    /// 设置分区 ID
    pub fn set_section_id(&mut self, section_id: Option<String>) {
        self.update_item(|item| {
            item.section_id = section_id;
        });
    }

    /// 设置优先级
    pub fn set_priority(&mut self, priority: i32) {
        self.update_item(|item| {
            item.priority = Some(priority);
        });
    }

    /// 设置截止日期
    pub fn set_due_date(&mut self, due_date: Option<todos::DueDate>) {
        self.update_item(|item| {
            item.due = due_date.map(|d| serde_json::to_value(d).unwrap_or_default());
        });
    }

    /// 设置内容
    pub fn set_content(&mut self, content: String) {
        self.update_item(|item| {
            item.content = content;
        });
    }

    /// 设置描述
    pub fn set_description(&mut self, description: Option<String>) {
        self.update_item(|item| {
            item.description = description;
        });
    }

    /// 设置完成状态
    pub fn set_completed(&mut self, completed: bool) {
        self.update_item(|item| {
            item.checked = completed;
            item.completed_at = if completed { Some(chrono::Utc::now().naive_utc()) } else { None };
        });
    }

    /// 设置置顶状态
    pub fn set_pinned(&mut self, pinned: bool) {
        self.update_item(|item| {
            item.pinned = pinned;
        });
    }
}
