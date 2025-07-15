use super::BaseObject;
use crate::entity::prelude::ItemEntity;
use crate::entity::{AttachmentModel, ItemModel, LabelModel, ProjectModel, ReminderModel, SectionModel};
use crate::enums::{ItemType, RecurrencyEndType, RecurrencyType, ReminderType, SourceType};
use crate::error::TodoError;
use crate::objects::{BaseTrait, DueDate};
use crate::{constants, utils, Store, Util};
use crate::{Reminder, Source};
use chrono::{Datelike, Local, NaiveDateTime};
use futures::{TryFutureExt, TryStreamExt};
use sea_orm::prelude::*;
use std::cmp::PartialEq;
use std::collections::HashMap;
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Item {
    pub model: ItemModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    label_count: Option<usize>,
    custom_order: bool,
    show_item: bool,
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
        self.model.priority.unwrap_or_else(|| constants::PRIORITY_4)
    }
    pub fn set_priority(&mut self, priority: i32) -> &mut Self {
        self.model.priority = Some(priority.into());
        self
    }
    pub fn labels(&self) -> Vec<LabelModel> {
        self.model
            .labels
            .as_ref()
            .and_then(|json_str| serde_json::from_value::<Vec<LabelModel>>(json_str.clone()).ok())
            .unwrap_or_default()
    }
    pub fn set_labels(&mut self, labels: Vec<LabelModel>) -> &mut Self {
        self.model.labels = Some(serde_json::value::to_value(labels).unwrap());
        self
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

impl PartialEq<&RecurrencyEndType> for RecurrencyEndType {
    fn eq(&self, other: &&RecurrencyEndType) -> bool {
        todo!()
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
            label_count: None,
            custom_order: false,
            show_item: false,
        }
    }
    pub async fn store(&self) -> &Store {
        self.store
            .get_or_init(|| async { Store::new(self.db.clone()).await })
            .await
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
    pub fn set_activate_name_editable(&self, v: bool) {
        todo!();
    }
    pub fn short_content(&self) -> String {
        Util::get_default().get_short_name(&*self.model.content, 0)
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
        if self.model.pinned {
            "planner-pin-tack"
        } else {
            "planner-pinned"
        }
    }

    pub fn has_due(&self) -> bool {
        self.due()
            .and_then(|d| Some(d.datetime().is_some()))
            .unwrap_or(false)
    }
    pub fn has_time(&self) -> bool {
        self.due()
            .and_then(|d| d.datetime())
            .map_or(false, |dt| utils::DateTime::default().has_time(&dt))
    }

    pub fn completed_date(&self) -> Option<NaiveDateTime> {
        self.model.completed_at.as_ref().and_then(|date|
            date.and_local_timezone(Local).single().map(|datetime|
                utils::DateTime::default().get_date_from_string(&datetime.naive_local().to_string())))
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
        self.model.added_at.and_local_timezone(Local).single().map(|dt|
            utils::DateTime::default().get_date_from_string(&dt.naive_local().to_string())).unwrap_or_default()
    }
    pub fn updated_datetime(&self) -> NaiveDateTime {
        self.model.updated_at.and_local_timezone(Local).single().map(|dt|
            utils::DateTime::default().get_date_from_string(&dt.naive_local().to_string())).unwrap_or_default()
    }

    pub async fn parent(&self) -> Option<ItemModel> {
        self.store()
            .await
            .get_item(&self.model.parent_id.as_ref()?)
            .await
    }
    pub async fn project(&self) -> Option<ProjectModel> {
        self.store()
            .await
            .get_project(&self.model.project_id.as_ref()?)
            .await
    }
    pub async fn section(&self) -> Option<SectionModel> {
        self.store()
            .await
            .get_section(&self.model.section_id.as_ref()?)
            .await
    }
    // subitems
    pub async fn items(&self) -> Vec<ItemModel> {
        let mut items = self.store().await.get_subitems(&self.model.id).await;
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }
    pub async fn items_uncomplete(&self) -> Vec<ItemModel> {
        self.store().await.get_subitems_uncomplete(&self.model.id).await
    }
    pub async fn reminders(&self) -> Vec<ReminderModel> {
        self.store().await.get_reminders_by_item(&self.model.id).await
    }
    pub async fn attachments(&self) -> Vec<AttachmentModel> {
        self.store().await.get_attachments_by_itemid(&self.model.id).await
    }
    pub(crate) fn has_labels(&self) -> bool {
        !self.labels().is_empty()
    }
    pub fn has_label(&self, id: &str) -> bool {
        self.get_label(id).is_some()
    }

    pub fn exists_project(&self, project: &ProjectModel) -> bool {
        self.model.project_id.as_deref().and_then(|id| id == project.id).unwrap_or(false);
        self.parent()
            .is_some_and(|parent| parent.exists_project(project))
    }
    pub fn get_label(&self, id: &str) -> Option<LabelModel> {
        self.labels().iter().find(|l| l.id == id).cloned()
    }
    pub fn get_label_by_name(&self, name: &str, labels_list: Vec<LabelModel>) -> Option<LabelModel> {
        labels_list.iter().find(|s| s.name == name).cloned()
    }

    pub async fn has_reminders(&self) -> bool {
        !self.reminders().await.is_empty()
    }

    pub fn get_caldav_categories(&self) {}
    pub fn check_labels(&mut self, new_labels: HashMap<String, LabelModel>) {
        for (key, label) in &new_labels {
            if self.get_label(&label.id).is_none() {
                self.add_label_if_not_exist(label.clone());
            }
        }
        for label in self.labels() {
            if !new_labels.contains_key(&label.id) {
                self.delete_item_label(&label.id);
            }
        }
    }
    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        self.items().await.iter().find(|i| i.id == id).cloned()
    }
    pub async fn add_item_if_not_exists(&self, item: &mut ItemModel) -> Result<ItemModel, TodoError> {
        match self.get_item(&item.id).await {
            Some(item) => Ok(item),
            None => {
                item.parent_id = Some(self.model.id.clone());
                self.store().await.insert_item(item.clone(), true).await
            }
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
    pub fn duplicate(&self) -> ItemModel {
        ItemModel {
            content: self.model.content.clone(),
            description: self.model.description.clone(),
            pinned: self.model.pinned,
            due: self.model.due.clone(),
            priority: self.model.priority,
            labels: self.model.labels.clone(),
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
                    format!("({})", utils::DateTime::default()
                        .get_relative_date_from_date(&datetime))
                })
                .unwrap_or_else(|| " ".to_string())
        }
    }
    fn add_label_if_not_exist(&self, label: LabelModel) {
        todo!()
    }
    pub fn get_labels_names(&self, labels: Vec<LabelModel>) -> String {
        labels.iter()
              .map(|l| l.name.as_str())
              .collect::<Vec<_>>()
              .join(",")
    }
    pub fn set_recurrency(&self, due_date: &DueDate) {
        let Some(mut due) = self.due() else { return };
        if due.is_recurrency_equal(due_date.clone()) { return; };

        let datetime_utils = utils::DateTime::default();

        match due_date.recurrency_type {
            RecurrencyType::MINUTELY | RecurrencyType::HOURLY if !self.has_due() => {
                due.date = datetime_utils.get_todoist_datetime_format(&Local::now().naive_local());
            }
            RecurrencyType::EveryDay | RecurrencyType::EveryMonth | RecurrencyType::EveryYear if !self.has_due() => {
                due.date = datetime_utils.get_todoist_datetime_format(&datetime_utils.get_today_format_date());
            }
            RecurrencyType::EveryWeek => {
                if due_date.has_weeks() {
                    let due_selected = self.has_due()
                                           .then(|| due.datetime())
                                           .flatten()
                                           .unwrap_or_else(|| datetime_utils.get_today_format_date());

                    let day_of_week = due_selected.weekday() as i32;
                    let next_day = utils::DateTime::get_next_day_of_week_from_recurrency_week(due_selected, due_date.clone())
                        .unwrap_or_default();

                    let new_date = if day_of_week == next_day {
                        due_selected
                    } else {
                        datetime_utils.next_recurrency_week(due_selected, due_date.clone(), false)
                    };

                    due.date = datetime_utils.get_todoist_datetime_format(&new_date);
                } else if !self.has_due() {
                    due.date = datetime_utils.get_todoist_datetime_format(&datetime_utils.get_today_format_date());
                }
            }
            _ => {}
        }

        due.is_recurring = due_date.is_recurring;
        due.recurrency_type = due_date.recurrency_type.clone();
        due.recurrency_interval = due_date.recurrency_interval;
        due.recurrency_weeks = due_date.recurrency_weeks.clone();
        due.recurrency_count = due_date.recurrency_count;
        due.recurrency_end = due_date.recurrency_end.clone();
    }
    pub async fn update_next_recurrency(&mut self) -> Result<(), TodoError> {
        let Some(mut due) = self.due() else { return Ok(()) };
        let datetime_utils = utils::DateTime::default();

        // Calculate next recurrence date
        let current_datetime = due.datetime().ok_or(TodoError::IDNotFound)?;
        let next_recurrency = datetime_utils.next_recurrency(current_datetime, due.clone());
        due.date = datetime_utils.get_todoist_datetime_format(&next_recurrency);

        // Update count for AFTER end type
        if due.end_type() == &RecurrencyEndType::AFTER {
            due.recurrency_count = due.recurrency_count.saturating_sub(1);
        }
        self.model.due = serde_json::to_value(&due).ok();
        self.store()
            .await
            .update_item(self.model.clone(), "")
            .await?;
        Ok(())
    }
    pub async fn add_item_label(&mut self, label_model: LabelModel) -> Result<LabelModel, TodoError> {
        let mut labels = self.labels();
        labels.insert(0, label_model.clone());
        self.model.labels = serde_json::to_value(&labels).ok();
        self.store().await.update_item(self.model.clone(), "").await?;
        self.store().await.insert_label(label_model.clone()).await
    }
    pub async fn delete_item(&self) -> Result<(), TodoError> {
        self.store().await.delete_item(&self.model.id).await
    }
    pub async fn delete_item_label(&mut self, id: &str) -> Result<u64, TodoError> {
        let labels_model = self.labels();
        let Some(label_model) = self.get_label(id) else {
            return Ok(0);
        };
        let labels: Vec<_> = labels_model.into_iter().filter(|l| l.id != id).collect();
        self.model.labels = serde_json::to_value(&labels).ok();
        self.store().await.update_item(self.model.clone(), "").await?;
        self.store().await.delete_label(id).await
    }
    pub async fn update_local(&self) {
        self.store().await.update_item(self.model.clone(), "").await;
    }
    pub async fn update(&self, update_id: &str) -> Result<(), TodoError> {
        // if (project.source_type == SourceType.LOCAL) {
        //     Services.Store.instance ().update_item (this, update_id);
        // } else if (project.source_type == SourceType.TODOIST) {
        //     Services.Todoist.get_default ().update.begin (this, (obj, res) => {
        //         Services.Todoist.get_default ().update.end (res);
        //         Services.Store.instance ().update_item (this, update_id);
        //     });
        // } else if (project.source_type == SourceType.CALDAV) {
        //     Services.CalDAV.Core.get_default ().add_task.begin (this, true, (obj, res) => {
        //         HttpResponse response = Services.CalDAV.Core.get_default ().add_task.end (res);
        //
        //         if (response.status) {
        //             Services.Store.instance ().update_item (this, update_id);
        //         }
        //     });
        // }

        self.store().await.update_item(self.model.clone(), &update_id).await?;
        Ok(())
    }
    pub async fn move_item(&self, project_id: &str, section_id: &str) -> Result<(), TodoError> {
        self.store().await.move_item(&self.model.id, project_id, section_id).await
    }
    pub async fn update_pin(&self) -> Result<(), TodoError> {
        self.store().await.update_item_pin(&self.model.id).await
    }
    pub async fn was_archived(&self) -> bool {
        let parent = self.parent().await;
        if parent.is_some() {
            parent.as_ref().map(async |p| {
                return Item::from_db(self.db.clone(), &self.model.id).await.ok().as_ref().and_then(|i| i.was_archived()).unwrap_or_default();
            })
        }
        let section = self.section().await;
    }
    fn source(&self) -> Option<Source> {
        self.project()
            .as_ref()
            .map_or(Some(Source::default()), |p| p.source())
    }
    pub fn add_reminder_events(&self, reminder: &Reminder) {
        // self.store.reminder_added(reminder);
        // self.store.reminders().add(reminder);
        // reminder.item().reminder_added(reminder);
        // _add_reminder(reminder);
    }
    pub fn remove_all_relative_reminders(&self) {
        self.reminders()
            .iter()
            .filter(|r| r.reminder_type() == ReminderType::RELATIVE)
            .for_each(|r| {
                r.delete();
            });
    }
    pub fn update_date(&mut self, date: &NaiveDateTime) {
        let mut my_due = self.due().clone();
        my_due.date = if *date == EMPTY_DATETIME {
            "".to_string()
        } else {
            DateTime::default().get_todoist_datetime_format(date)
        };
        self.update_due(&mut my_due);
    }
    pub fn update_sync(&self, update_id: &str) {
        if let Some(project) = self.project() {
            match project.source_type() {
                SourceType::LOCAL => {
                    self.store.update_item(self, update_id);
                }
                SourceType::TODOIST => {
                    // Services.Todoist.get_default ().update_item (this, update_id);
                }
                SourceType::CALDAV => {
                    // Services.CalDAV.Core.get_default ().update_item (this, update_id);
                }
                _ => {}
            }
        }
    }
    pub fn update_due(&mut self, due: &mut DueDate) {
        let mut my_due = self.due().clone();
        my_due.date = due.date.clone();
        if self.has_time() {
            self.remove_all_relative_reminders();
            let mut reminder = Reminder::new();
            reminder.set_mm_offset(Util::get_default().get_reminders_mm_offset());
            reminder.set_reminder_type(&ReminderType::RELATIVE);
            self.add_reminder(&mut reminder);
        }
        if due.date.is_empty() {
            due.reset();
            self.remove_all_relative_reminders();
        }
        if !self.has_time() {
            self.remove_all_relative_reminders();
        }
        self.update_sync("");
    }
    pub async fn get_reminder(&self, reminder: &ReminderModel) -> Option<ReminderModel> {
        self.reminders().await.iter().find(|r| r.due == reminder.due).cloned()
    }
    pub async fn add_reminder_if_not_exists(&self, reminder: &ReminderModel) -> Result<ReminderModel, TodoError> {
        match self.get_reminder(reminder).await {
            Some(reminder) => { return Ok(reminder) }
            None => {
                self.store().await.insert_reminder(reminder.clone()).await
            }
        }
    }
    pub async fn get_attachment(&self, attachment: &AttachmentModel) -> Option<AttachmentModel> {
        self.attachments().await.iter().find(|a| a.file_path == attachment.file_path).cloned()
    }
    pub async fn add_attachment_if_not_exists(&self, attachment: &AttachmentModel) -> Result<AttachmentModel, TodoError> {
        match self.get_attachment(attachment).await {
            Some(attachment) => { Ok(attachment) }
            None => { self.store().await.insert_attachment(attachment.clone()).await }
        }
    }

    pub async fn add_label_if_not_exists(&self, label: &LabelModel) -> Result<LabelModel, TodoError> {
        match self.get_label(&label.id) {
            Some(label) => { Ok(label) }
            None => {
                self.store().await.insert_label(label.clone()).await
            }
        }
    }
    pub fn add_reminder(&self, reminder: &mut ReminderModel) {
        reminder.item_id = Some(self.model.id.clone());
        self.add_reminder_if_not_exists(reminder);
    }

    pub fn complete_item(&self) {
        if let Some(project) = self.project() {
            match project.source_type() {
                SourceType::LOCAL => {
                    self.store.complete_item(self);
                }
                SourceType::TODOIST => {
                    // Services.Todoist.get_default ().complete_item (this)
                }
                SourceType::CALDAV => {
                    // Services.CalDAV.Core.get_default ().complete_item (this);
                    // foreach (Objects.Item subitem in Services.Store.instance ().get_subitems (this)) {
                    //     subitem.checked = checked;
                    //     subitem.completed_at = completed_at;
                    //     subitem.complete_item.begin (old_checked);
                    // }
                }
                _ => {}
            }
        }
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
