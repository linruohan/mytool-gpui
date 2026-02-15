use std::collections::HashMap;

use chrono::{Datelike, Local, NaiveDateTime};
use futures::{TryFutureExt, TryStreamExt};
use sea_orm::prelude::*;
use tokio::sync::OnceCell;

use super::{BaseObject, Project, Section};
use crate::{
    Reminder, Store, Util, constants,
    entity::{
        AttachmentModel, ItemModel, LabelModel, ProjectModel, ReminderModel, SectionModel,
        SourceModel, prelude::ItemEntity,
    },
    enums::{ItemType, RecurrencyEndType, RecurrencyType, ReminderType},
    error::TodoError,
    objects::{BaseTrait, DueDate},
    utils,
};

#[derive(Clone, Debug)]
pub struct Item {
    pub model: ItemModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    labels: OnceCell<Vec<LabelModel>>,
    attachments: OnceCell<Vec<AttachmentModel>>,
    reminders: OnceCell<Vec<ReminderModel>>,
    subitems: OnceCell<Vec<ItemModel>>,
    label_count: Option<usize>,
    custom_order: bool,
    show_item: bool,
}

impl Drop for Item {
    fn drop(&mut self) {
        // Clean up any resources here if needed
        // The database connection will be automatically cleaned up
        // as it's managed by the connection pool
    }
}
impl Item {
    pub fn due(&self) -> Option<DueDate> {
        self.model
            .due
            .as_ref()
            .map(|json_str| serde_json::from_value::<DueDate>(json_str.clone()).ok())
            .unwrap_or_default()
    }

    pub fn set_due(&mut self, due: DueDate) -> &mut Self {
        self.model.due = Some(serde_json::value::to_value(due).unwrap());
        self
    }

    pub fn priority(&self) -> i32 {
        self.model.priority.unwrap_or(constants::PRIORITY_4)
    }

    pub fn set_priority(&mut self, priority: i32) -> &mut Self {
        self.model.priority = Some(priority);
        self
    }

    /// 获取 Item 的所有 Labels
    ///
    /// 使用 item_labels 关联表查询，替代原有的分号分隔字符串存储
    pub async fn labels(&self) -> Vec<LabelModel> {
        self.labels
            .get_or_init(|| async {
                // 使用 Store 通过关联表查询 Labels
                match self.store().await.get_labels_by_item(&self.model.id).await {
                    Ok(labels) => labels,
                    Err(e) => {
                        tracing::error!("Failed to get labels for item {}: {:?}", self.model.id, e);
                        vec![]
                    },
                }
            })
            .await
            .clone()
    }

    /// 设置 Item 的 Labels
    ///
    /// 注意：此方法仅更新本地缓存，要持久化到数据库请使用 add_label/delete_label
    pub fn set_labels(&mut self, labels: Vec<LabelModel>) -> &mut Self {
        // 更新本地缓存
        self.labels = OnceCell::from(labels);
        self
    }

    /// 添加 Label 到 Item
    ///
    /// 通过 Store 使用 item_labels 关联表添加关系
    pub async fn add_label(&mut self, label: &LabelModel) -> Result<LabelModel, TodoError> {
        let store = self.store().await;

        // 检查是否已存在
        if store.item_has_label(&self.model.id, &label.id).await? {
            return Ok(label.clone());
        }

        // 添加关联
        store.add_label_to_item(&self.model.id, &label.name).await?;

        // 刷新缓存
        self.labels = OnceCell::new();

        Ok(label.clone())
    }

    /// 从 Item 删除 Label
    ///
    /// 通过 Store 使用 item_labels 关联表删除关系
    pub async fn delete_label(&mut self, label_id: &str) -> Result<(), TodoError> {
        let store = self.store().await;

        // 删除关联
        store.remove_label_from_item(&self.model.id, label_id).await?;

        // 刷新缓存
        self.labels = OnceCell::new();

        Ok(())
    }

    pub fn extra_data(&self) -> Option<serde_json::Value> {
        self.model.extra_data.clone()
    }

    pub fn set_extra_data(&mut self, extra_data: Option<serde_json::Value>) -> &mut Self {
        self.model.extra_data = extra_data;
        self
    }

    pub fn item_type(&self) -> Option<ItemType> {
        self.model
            .item_type
            .as_deref()
            .map(|item_type| serde_json::from_str::<ItemType>(item_type).ok())
            .unwrap_or(Some(ItemType::TASK))
    }

    pub fn set_item_type(&mut self, item_type: ItemType) -> &mut Self {
        self.model.item_type = Some(serde_json::to_string(&item_type).unwrap());
        self
    }
}

impl Item {
    pub fn new(db: DatabaseConnection, model: ItemModel) -> Self {
        let base = BaseObject::default();
        Self {
            model,
            base,
            db,
            store: OnceCell::new(),
            labels: OnceCell::new(),
            attachments: OnceCell::new(),
            reminders: OnceCell::new(),
            subitems: OnceCell::new(),
            label_count: None,
            custom_order: false,
            show_item: false,
        }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async { Store::new(self.db.clone()) }).await
    }

    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = ItemEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }

    pub fn activate_name_editable(&self) -> bool {
        false
    }

    pub fn set_activate_name_editable(&mut self, activate_name_editable: bool) -> bool {
        activate_name_editable
    }

    pub fn short_content(&self) -> String {
        Util::get_default().get_short_name(&self.model.content, 0)
    }

    pub fn priority_icon(&self) -> &str {
        match self.model.priority.as_ref() {
            Some(&constants::PRIORITY_1) => "priority-icon-1",
            Some(&constants::PRIORITY_2) => "priority-icon-2",
            Some(&constants::PRIORITY_3) => "priority-icon-3",
            _ => "planner-flag",
        }
    }

    pub fn priority_color(&self) -> &str {
        match self.model.priority.as_ref() {
            Some(&constants::PRIORITY_1) => "#ff7066",
            Some(&constants::PRIORITY_2) => "#ff9914",
            Some(&constants::PRIORITY_3) => "#5297ff",
            _ => "@text_color",
        }
    }

    pub fn priority_text(&self) -> &str {
        match self.model.priority.as_ref() {
            Some(&constants::PRIORITY_1) => "Priority 1: high",
            Some(&constants::PRIORITY_2) => "Priority 2: medium",
            Some(&constants::PRIORITY_3) => "Priority 3: low",
            _ => "Priority 4: none",
        }
    }

    pub fn pinned_icon(&self) -> &str {
        if self.model.pinned { "planner-pin-tack" } else { "planner-pinned" }
    }

    pub fn has_due(&self) -> bool {
        self.due().map(|d| d.datetime().is_some()).unwrap_or(false)
    }

    pub fn has_time(&self) -> bool {
        self.due()
            .and_then(|d| d.datetime())
            .is_some_and(|dt| utils::DateTime::default().has_time(&dt))
    }

    pub fn completed_date(&self) -> Option<NaiveDateTime> {
        self.model.completed_at.as_ref().and_then(|date| {
            date.and_local_timezone(Local).single().map(|datetime| {
                utils::DateTime::default().get_date_from_string(&datetime.naive_local().to_string())
            })
        })
    }

    pub async fn has_parent(&self) -> bool {
        let Some(id) = self.model.parent_id.as_deref() else {
            return false;
        };
        self.store().await.get_item(id).await.is_some()
    }

    pub async fn has_section(&self) -> bool {
        let Some(id) = self.model.section_id.as_deref() else {
            return false;
        };
        self.store().await.get_section(id).await.is_some()
    }

    pub fn show_item(&self) -> bool {
        self.show_item
    }

    pub fn set_show_item(&mut self, show_item: bool) -> &mut Self {
        self.show_item = show_item;
        self
    }

    pub fn ics(&self) -> &str {
        // Services.Todoist.get_default ().get_string_member_by_object (extra_data, "ics")
        ""
    }

    pub fn added_datetime(&self) -> NaiveDateTime {
        self.model
            .added_at
            .and_local_timezone(Local)
            .single()
            .map(|dt| {
                utils::DateTime::default().get_date_from_string(&dt.naive_local().to_string())
            })
            .unwrap_or_default()
    }

    pub fn updated_datetime(&self) -> NaiveDateTime {
        self.model
            .updated_at
            .and_local_timezone(Local)
            .single()
            .map(|dt| {
                utils::DateTime::default().get_date_from_string(&dt.naive_local().to_string())
            })
            .unwrap_or_default()
    }

    pub async fn parent(&self) -> Option<ItemModel> {
        self.store().await.get_item(self.model.parent_id.as_ref()?).await
    }

    pub async fn project(&self) -> Option<ProjectModel> {
        self.store().await.get_project(self.model.project_id.as_ref()?).await
    }

    pub async fn section(&self) -> Option<SectionModel> {
        self.store().await.get_section(self.model.section_id.as_ref()?).await
    }

    // subitems
    pub async fn items(&self) -> Vec<ItemModel> {
        self.subitems
            .get_or_init(|| async {
                let mut items =
                    self.store().await.get_subitems(&self.model.id).await.unwrap_or_default();
                items.sort_by_key(|a| a.child_order);
                items
            })
            .await
            .clone()
    }

    pub async fn items_uncomplete(&self) -> Vec<ItemModel> {
        // 暂时返回空向量，因为不存在 get_subitems_uncomplete 方法
        vec![]
    }

    pub async fn reminders(&self) -> Vec<ReminderModel> {
        self.reminders
            .get_or_init(|| async {
                self.store().await.get_reminders_by_item(&self.model.id).await.unwrap_or_default()
            })
            .await
            .clone()
    }

    pub async fn attachments(&self) -> Vec<AttachmentModel> {
        self.attachments
            .get_or_init(|| async {
                // 暂时返回空向量，因为不存在 get_attachments_by_itemid 方法
                vec![]
            })
            .await
            .clone()
    }

    pub async fn has_labels(&self) -> bool {
        !self.labels().await.is_empty()
    }

    pub async fn has_label(&self, id: &str) -> bool {
        self.get_label(id).await.is_some()
    }

    pub async fn exists_project(&self, project: ProjectModel) -> bool {
        Box::pin(async move {
            if let Some(p) = self.parent().await.as_ref()
                && let Ok(item) = Item::from_db(self.db.clone(), &self.model.id).await
            {
                return item.exists_project(project).await;
            }
            self.model.project_id == Some(project.id)
        })
        .await
    }

    pub async fn get_label(&self, id: &str) -> Option<LabelModel> {
        self.labels().await.iter().find(|l| l.id == id).cloned()
    }

    pub fn get_label_by_name(
        &self,
        name: &str,
        labels_list: Vec<LabelModel>,
    ) -> Option<LabelModel> {
        labels_list.iter().find(|s| s.name == name).cloned()
    }

    pub async fn has_reminders(&self) -> bool {
        !self.reminders().await.is_empty()
    }

    pub fn get_caldav_categories(&self) {}

    pub async fn check_labels(&mut self, new_labels: HashMap<String, LabelModel>) {
        for (key, label) in &new_labels {
            if self.get_label(&label.id).await.is_none() {
                self.add_label_if_not_exists(label);
            }
        }
        for label in self.labels().await {
            if !new_labels.contains_key(&label.id) {
                self.delete_item_label(&label.id);
            }
        }
    }

    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        self.items().await.iter().find(|i| i.id == id).cloned()
    }

    pub async fn add_item_if_not_exists(
        &self,
        item: &mut ItemModel,
    ) -> Result<ItemModel, TodoError> {
        match self.get_item(&item.id).await {
            Some(item) => Ok(item),
            None => {
                item.parent_id = Some(self.model.id.clone());
                self.store().await.insert_item(item.clone(), true).await
            },
        }
    }

    pub fn generate_copy(&self) -> ItemModel {
        ItemModel {
            content: self.model.content.clone(),
            description: self.model.description.clone(),
            pinned: self.model.pinned,
            due: self.model.due.clone(),
            priority: self.model.priority,
            ..Default::default()
        }
    }

    /// 复制 Item（创建副本）
    ///
    /// 注意：Labels 需要通过 item_labels 关联表单独处理，不会自动复制
    pub fn duplicate(&self) -> ItemModel {
        ItemModel {
            content: self.model.content.clone(),
            description: self.model.description.clone(),
            pinned: self.model.pinned,
            due: self.model.due.clone(),
            priority: self.model.priority,
            // Labels 现在存储在 item_labels 关联表中，不在 ItemModel 中
            ..Default::default()
        }
    }

    pub fn get_format_date(&self) -> String {
        if !self.has_due() {
            " ".to_string()
        } else {
            self.due()
                .as_ref()
                .and_then(|dt| dt.datetime())
                .map(|datetime| {
                    format!(
                        "({})",
                        utils::DateTime::default().get_relative_date_from_date(&datetime)
                    )
                })
                .unwrap_or_else(|| " ".to_string())
        }
    }

    pub fn get_labels_names(&self, labels: Vec<LabelModel>) -> String {
        labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(",")
    }

    pub fn set_recurrency(&self, due_date: &DueDate) {
        let Some(mut due) = self.due() else { return };
        if due.is_recurrency_equal(due_date.clone()) {
            return;
        };

        let datetime_utils = utils::DateTime::default();

        match due_date.recurrency_type {
            RecurrencyType::MINUTELY | RecurrencyType::HOURLY if !self.has_due() => {
                due.date = datetime_utils.get_todoist_datetime_format(&Local::now().naive_local());
            },
            RecurrencyType::EveryDay | RecurrencyType::EveryMonth | RecurrencyType::EveryYear
                if !self.has_due() =>
            {
                due.date = datetime_utils
                    .get_todoist_datetime_format(&datetime_utils.get_today_format_date());
            },
            RecurrencyType::EveryWeek => {
                if due_date.has_weeks() {
                    let due_selected = self
                        .has_due()
                        .then(|| due.datetime())
                        .flatten()
                        .unwrap_or_else(|| datetime_utils.get_today_format_date());

                    let day_of_week = due_selected.weekday() as i32;
                    let next_day = utils::DateTime::get_next_day_of_week_from_recurrency_week(
                        due_selected,
                        due_date.clone(),
                    )
                    .unwrap_or_default();

                    let new_date = if day_of_week == next_day {
                        due_selected
                    } else {
                        datetime_utils.next_recurrency_week(due_selected, due_date.clone(), false)
                    };

                    due.date = datetime_utils.get_todoist_datetime_format(&new_date);
                } else if !self.has_due() {
                    due.date = datetime_utils
                        .get_todoist_datetime_format(&datetime_utils.get_today_format_date());
                }
            },
            _ => {},
        }

        due.is_recurring = due_date.is_recurring;
        due.recurrency_type = due_date.recurrency_type.clone();
        due.recurrency_interval = due_date.recurrency_interval;
        due.recurrency_weeks = due_date.recurrency_weeks.clone();
        due.recurrency_count = due_date.recurrency_count;
        due.recurrency_end = due_date.recurrency_end.clone();
    }

    pub async fn update_next_recurrency(&mut self) -> Result<(), TodoError> {
        let Some(mut due) = self.due() else {
            return Ok(());
        };
        let datetime_utils = utils::DateTime::default();

        // Calculate next recurrence date
        let current_datetime = due.datetime().ok_or(TodoError::IDNotFound)?;
        let next_recurrency = datetime_utils.next_recurrency(current_datetime, due.clone());
        due.date = datetime_utils.get_todoist_datetime_format(&next_recurrency);

        // Update count for AFTER end type
        if due.end_type() == RecurrencyEndType::AFTER {
            due.recurrency_count = due.recurrency_count.saturating_sub(1);
        }
        self.model.due = serde_json::to_value(&due).ok();
        self.store().await.update_item(self.model.clone(), "").await?;
        Ok(())
    }

    /// 添加 Label 到 Item（使用 item_labels 关联表）
    ///
    /// 替代原有的字符串存储方式，现在使用关联表维护关系
    pub async fn add_item_label(
        &mut self,
        label_model: LabelModel,
    ) -> Result<LabelModel, TodoError> {
        // 使用新的 add_label 方法（通过关联表）
        self.add_label(&label_model).await?;

        // 同时插入 Label 到 labels 表（如果不存在）
        self.store().await.insert_label(label_model.clone()).await
    }

    pub async fn delete_item(&self) -> Result<(), TodoError> {
        self.store().await.delete_item(&self.model.id).await
    }

    /// 从 Item 删除 Label（使用 item_labels 关联表）
    ///
    /// 替代原有的字符串存储方式，现在使用关联表维护关系
    pub async fn delete_item_label(&mut self, id: &str) -> Result<u64, TodoError> {
        // 检查 Label 是否存在
        let Some(_) = self.get_label(id).await else {
            return Ok(0);
        };

        // 使用新的 delete_label 方法（通过关联表）
        self.delete_label(id).await?;
        Ok(1)
    }

    pub async fn update_local(&self) {
        self.store().await.update_item(self.model.clone(), "").await;
    }

    pub async fn update(&self, update_id: &str) -> Result<ItemModel, TodoError> {
        self.store().await.update_item(self.model.clone(), update_id).await
    }

    pub async fn move_item(&self, project_id: &str, section_id: &str) -> Result<(), TodoError> {
        self.store().await.move_item(&self.model.id, project_id, section_id).await
    }

    pub async fn update_pin(&self) -> Result<(), TodoError> {
        self.store().await.update_item_pin(&self.model.id, true).await
    }

    pub async fn was_archived(&self) -> bool {
        Box::pin(async move {
            if let Some(p) = self.parent().await.as_ref()
                && let Ok(item) = Item::from_db(self.db.clone(), &self.model.id).await
            {
                return item.was_archived().await;
            }

            if let Some(s) = self.section().await.as_ref()
                && let Ok(sec) = Section::from_db(self.db.clone(), &s.id).await
            {
                return sec.was_archived().await;
            }
            false
        })
        .await
    }

    pub async fn source(&self) -> Option<SourceModel> {
        if let Some(project_model) = self.project().await.as_ref()
            && let Ok(project) = Project::from_db(self.db.clone(), &project_model.id).await
        {
            return project.source().await;
        }
        None
    }

    pub fn add_reminder_events(&self, reminder: &Reminder) {
        // self.store.reminder_added(reminder);
        // self.store.reminders().add(reminder);
        // reminder.item().reminder_added(reminder);
        // _add_reminder(reminder);
    }

    pub async fn remove_all_relative_reminders(&self) -> Result<(), TodoError> {
        let reminders = self.reminders().await;
        let store = self.store().await;

        for r in
            reminders.iter().filter(|r| r.reminder_type == Some(ReminderType::RELATIVE.to_string()))
        {
            store.delete_reminder(&r.id).await?;
        }
        Ok(())
    }

    pub async fn update_date(&mut self, date: &NaiveDateTime) {
        if let Some(mut due) = self.due() {
            due.date = utils::DateTime::default().get_todoist_datetime_format(date);
            self.update_due(due).await; // Move due into update_due
        }
    }

    pub async fn update_sync(&self, update_id: &str) {
        self.store().await.update_item(self.model.clone(), update_id);
    }

    pub async fn update_due(&mut self, due_date: DueDate) {
        if self.has_time() {
            self.remove_all_relative_reminders().await.ok();
            let mut reminder = ReminderModel {
                reminder_type: Some(ReminderType::RELATIVE.to_string()),
                mm_offset: Some(Util::get_default().get_reminders_mm_offset()),
                ..Default::default()
            };
            self.add_reminder(&mut reminder).await.ok();
        }
        if let Some(mut due) = self.due() {
            due.date = due_date.date.to_string();

            if due.date.is_empty() {
                due.reset();
                self.remove_all_relative_reminders().await.ok();
            }
        }
        if !self.has_time() {
            self.remove_all_relative_reminders().await.ok();
        }
        self.update_sync("").await;
    }

    pub async fn get_reminder(&self, reminder: &ReminderModel) -> Option<ReminderModel> {
        self.reminders().await.iter().find(|r| r.due == reminder.due).cloned()
    }

    pub async fn add_reminder_if_not_exists(
        &self,
        reminder: &ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        match self.get_reminder(reminder).await {
            Some(reminder) => Ok(reminder),
            None => self.store().await.insert_reminder(reminder.clone()).await,
        }
    }

    pub async fn get_attachment(&self, attachment: &AttachmentModel) -> Option<AttachmentModel> {
        self.attachments().await.iter().find(|a| a.file_path == attachment.file_path).cloned()
    }

    pub async fn add_attachment_if_not_exists(
        &self,
        attachment: &AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        match self.get_attachment(attachment).await {
            Some(attachment) => Ok(attachment),
            None => {
                // 暂时返回附件本身，因为不存在 insert_attachment 方法
                Ok(attachment.clone())
            },
        }
    }

    pub async fn add_label_if_not_exists(
        &self,
        label: &LabelModel,
    ) -> Result<LabelModel, TodoError> {
        match self.get_label(&label.id).await {
            Some(label) => Ok(label),
            None => self.store().await.insert_label(label.clone()).await,
        }
    }

    pub async fn add_reminder(
        &self,
        reminder: &mut ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        reminder.item_id = Some(self.model.id.clone());
        self.add_reminder_if_not_exists(reminder).await
    }

    pub async fn complete_item(&self) {
        self.store().await.complete_item(&self.model.id, true, true).await;
    }
}

impl BaseTrait for Item {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
