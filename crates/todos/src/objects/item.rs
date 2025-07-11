use super::{Attachment, BaseObject, Label};
use crate::entity::items::Model as ItemModel;
use crate::entity::prelude::ItemEntity;
use crate::entity::{LabelModel, ProjectModel, SectionModel};
use crate::enums::{ItemType, ReminderType, SourceType};
use crate::error::TodoError;
use crate::objects::{BaseTrait, DueDate};
use crate::{Project, Reminder, Source};
use crate::{Store, Util, constants, utils};
use chrono::NaiveDateTime;
use sea_orm::prelude::*;
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
    pub fn labels(&self) -> Option<Vec<LabelModel>> {
        self.model
            .labels
            .as_ref()
            .map(|json_str| serde_json::from_value::<Vec<LabelModel>>(json_str.clone()).ok())
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
            .unwrap_or_default()
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

    pub fn set_custom_order(&mut self, order: bool) -> &mut Self {
        self.custom_order = order;
        self
    }
    pub fn pinned_icon(&self) -> &str {
        if self.model.pinned {
            "planner-pin-tack"
        } else {
            "planner-pinned"
        }
    }

    pub fn completed(&self) -> bool {
        self.model.checked
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
        self.model.completed_at.as_ref().and_then(|date| {
            let local_date = date.and_local_timezone(&Local).to_string();
            Some(utils::DateTime::default().get_date_from_string(&local_date))
        })
    }

    pub async fn has_parent(&self) -> bool {
        match self.parent_id().as_deref() {
            Some(parent_id) => self
                .store
                .get_item(parent_id)
                .await
                .map_or(false, |item| item.is_some()),
            None => false,
        }
    }
    pub fn has_section(&self) -> bool {
        self.section_id().as_deref().map_or(false, async move |id| {
            self.store.get_section(id).await.is_some()
        })
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
        let local_date = self.model.added_at.and_local_timezone(&Local).to_string();
        utils::DateTime::default().get_date_from_string(&local_date)
    }
    pub fn updated_datetime(&self) -> NaiveDateTime {
        let local_date = self.model.updated_at.and_local_timezone(&Local).to_string();
        utils::DateTime::default().get_date_from_string(&local_date)
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
    pub fn items(&self) -> Vec<ItemModel> {
        let mut items = self.store.get_subitems(self.id());
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }
    pub fn items_uncomplete(&self) -> Vec<ItemModel> {
        self.store.get_subitems_uncomplete(self)
    }
    pub fn reminders(&self) -> Vec<Reminder> {
        self.store.get_reminders_by_item(self)
    }
    pub fn attachments(&self) -> Vec<Attachment> {
        self.store.get_attachments_by_item(self)
    }
    pub(crate) fn has_labels(&self) -> bool {
        !self.labels().is_empty()
    }
    pub fn has_label(&self, id: &str) -> bool {
        self.get_label(id).is_some()
    }

    pub fn exists_project(&self, project: &Project) -> bool {
        if project.id == self.project_id {
            return true;
        }
        self.parent()
            .is_some_and(|parent| parent.exists_project(project))
    }
    pub fn get_label(&self, id: &str) -> Option<Label> {
        self.labels().iter().find(|l| l.id == id).cloned()
    }
    pub fn get_label_by_name(&self, name: &str, labels_list: Vec<Label>) -> Option<Label> {
        labels_list.iter().find(|s| s.name == name).cloned()
    }

    pub fn has_reminders(&self) -> bool {
        !self.reminders().is_empty()
    }

    pub fn get_caldav_categories(&self) {}
    pub fn check_labels(&self, new_labels: HashMap<String, Label>) {
        for (key, label) in &new_labels {
            let label_id = label.id();
            if self.get_label(label_id).is_none() {
                self.add_label_if_not_exist(label.clone());
            }
        }
        for label in self.labels() {
            let label_id = label.id();
            if !new_labels.contains_key(label_id) {
                self.delete_item_label(label_id);
            }
        }
    }
    pub fn set_section(&mut self, section_id: &str) {
        self.model.section_id = Some(section_id.to_string());
    }
    pub fn set_project(&mut self, project: Project) {
        self.project_id = project.id.clone();
    }
    pub fn set_parent(&mut self, parent: Item) {
        self.parent_id = parent.id.clone();
    }

    fn add_label_if_not_exist(&self, label: Label) {
        todo!()
    }

    pub async fn delete_item_label(&self, id: &str) -> Result<(), TodoError> {
        todo!()
    }
    pub async fn update_local(&self) {
        self.store.update_item(self, "").await;
    }
    pub fn update(&self, update_id: &str) {
        if let Some(project) = self.project()
            && project.source_type() == SourceType::LOCAL
        {
            self.store.update_item(self, update_id);
        }
    }
    pub fn was_archived(&self) -> bool {
        self.parent()
            .map(|p| p.was_archived())
            .or_else(|| self.section().map(|s| s.was_archived()))
            .unwrap_or_else(|| self.project().is_some_and(|p| p.is_archived()))
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
    fn get_reminder(&self, reminder: &Reminder) -> Option<Reminder> {
        self.reminders()
            .iter()
            .find(|r| r.datetime() == reminder.datetime())
            .cloned()
    }
    fn add_reminder_if_not_exists(&self, reminder: &Reminder) {
        if self.get_reminder(reminder).is_none() {
            self.store.insert_reminder(reminder);
        }
    }
    pub fn add_reminder(&self, reminder: &mut Reminder) {
        reminder.item_id = self.id.clone();
        if let Some(project) = self.project() {
            match project.source_type() {
                SourceType::LOCAL => {
                    self.add_reminder_if_not_exists(reminder);
                }
                SourceType::TODOIST => {
                    // Services.Todoist.get_default ().add.begin (reminder, (obj, res) => {
                    //     HttpResponse response = Services.Todoist.get_default ().add.end (res);
                    //     loading = false;

                    //     if (response.status) {
                    //         reminder.id = response.data;
                    //     } else {
                    //         reminder.id = Util.get_default ().generate_id (reminder);
                    //     }

                    //     add_reminder_if_not_exists (reminder);
                    // });
                }
                _ => {}
            }
        }
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
