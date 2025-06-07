use crate::enums::{ItemType, ReminderType, SourceType};
use crate::objects::{BaseTrait, DueDate, ToBool};
use crate::schema::items;
use crate::services::store;
use crate::utils::{self, EMPTY_DATETIME};
use crate::{
    Attachment, Label, Project, Reminder, Section, Source, Store, Util, constants,
    generate_accessors,
};
use chrono::{Local, NaiveDateTime};
use derive_builder::Builder;
use diesel::Queryable;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(
    QueryableByName,
    Builder,
    Insertable,
    Clone,
    Queryable,
    PartialEq,
    Eq,
    Selectable,
    Serialize,
    Debug,
)]
#[diesel(table_name = items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Item {
    pub id: Option<String>,
    #[builder(default, setter(strip_option))]
    pub content: String,
    #[builder(default, setter(strip_option))]
    pub description: Option<String>,
    #[builder(default=Some(DueDate::default().to_string()), setter(strip_option))]
    pub due: Option<String>,
    #[builder(default=Some(Local::now().naive_local().to_string()), setter(strip_option))]
    pub added_at: Option<String>,
    #[builder(default, setter(strip_option))]
    pub completed_at: Option<String>,
    #[builder(default, setter(strip_option))]
    pub updated_at: Option<String>,
    #[builder(default, setter(strip_option))]
    pub section_id: Option<String>,
    #[builder(default, setter(strip_option))]
    pub project_id: Option<String>,
    #[builder(default, setter(strip_option))]
    pub parent_id: Option<String>,
    #[builder(default=Some(constants::PRIORITY_1), setter(strip_option))]
    pub priority: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub child_order: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub checked: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub is_deleted: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub day_order: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub collapsed: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub pinned: Option<i32>,
    #[builder(default, setter(strip_option))]
    pub labels: Option<String>,
    #[builder(default, setter(strip_option))]
    pub extra_data: Option<String>,
    #[builder(default=Some(ItemType::TASK.to_string()), setter(strip_option))]
    pub item_type: Option<String>,
}

impl Item {
    pub fn item_label_added(&self, label: &Label) {}
    pub fn item_label_deleted(&self, label: &Label) {}
    pub fn item_added(&self, item: &Item) {}
    pub fn reminder_added(&self, reminder: &Reminder) {}
    pub fn reminder_deleted(&self, reminder: &Reminder) {}
    pub fn show_item_changed(&self) {}
    pub fn collapsed_change(&self) {}
    pub fn attachment_added(&self, attachment: &Attachment) {}
    pub fn attachment_deleted(&self, attachment: &Attachment) {}
    pub fn pin_updated(&self) {}
}

impl Item {
    generate_accessors!(content:String);
    generate_accessors!(description: Option<String>);
    generate_accessors!(@due due: Option<String>);
    generate_accessors!(@nativedatetime added_at: Option<String>);
    generate_accessors!(@nativedatetime completed_at: Option<String>);
    generate_accessors!(@nativedatetime updated_at: Option<String>);
    generate_accessors!(section_id: Option<String>);
    generate_accessors!(project_id: Option<String>);
    generate_accessors!(parent_id: Option<String>);
    // generate_accessors!(priority: Option<i32>);
    pub fn priority(&self) -> i32 {
        self.priority.unwrap_or(constants::PRIORITY_4)
    }
    pub fn set_priority(&mut self, priority: i32) {
        self.priority = Some(priority);
    }
    generate_accessors!(child_order: Option<i32>);
    generate_accessors!(@bool checked: Option<i32>);
    generate_accessors!(@bool is_deleted: Option<i32>);
    generate_accessors!(day_order: Option<i32>);
    generate_accessors!(@bool collapsed: Option<i32>);
    generate_accessors!(@bool pinned: Option<i32>);
    generate_accessors!(@labels labels: Option<String>);
    generate_accessors!(extra_data: Option<String>);
    // generate_accessors!(item_type: Option<String>);
    pub fn item_type(&self) -> ItemType {
        self.item_type
            .as_deref()
            .and_then(|s| serde_json::from_str::<ItemType>(s).ok())
            .unwrap_or(ItemType::TASK)
    }
    pub fn set_item_type(&mut self, item_type: ItemType) {
        self.item_type = Some(item_type.to_string())
    }

    pub fn activate_name_editable(&self) -> bool {
        false
    }
    pub fn set_activate_name_editable(&self, v: bool) {
        todo!();
    }

    pub(crate) fn has_labels(&self) -> bool {
        self.labels().len() > 0
    }
    pub fn has_label(&self, id: &str) -> bool {
        self.get_label(id).is_some()
    }

    pub fn exists_project(&self, project: &Project) -> bool {
        if project.id == self.project_id {
            return true;
        }
        self.parent()
            .map_or(false, |parent| parent.exists_project(project))
    }
    pub fn get_label(&self, id: &str) -> Option<Label> {
        self.labels()
            .iter()
            .find(|l| l.id.as_deref() == Some(id))
            .cloned()
    }
    pub fn get_label_by_name(&self, name: &str, labels_list: Vec<Label>) -> Option<Label> {
        labels_list
            .iter()
            .find(|s| s.name.as_deref() == Some(name))
            .cloned()
    }
    pub fn short_content(&self) -> String {
        Util::get_default().get_short_name(self.content.clone(), 0)
    }
    pub fn priority_icon(&self) -> &str {
        match self.priority {
            Some(constants::PRIORITY_1) => "priority-icon-1",
            Some(constants::PRIORITY_2) => "priority-icon-2",
            Some(constants::PRIORITY_3) => "priority-icon-3",
            _ => "planner-flag",
        }
    }
    pub fn has_reminders(&self) -> bool {
        self.reminders().len() > 0
    }

    pub fn priority_color(&self) -> &str {
        match self.priority {
            Some(constants::PRIORITY_1) => "#ff7066",
            Some(constants::PRIORITY_2) => "#ff9914",
            Some(constants::PRIORITY_3) => "#5297ff",
            _ => "@text_color",
        }
    }

    pub fn priority_text(&self) -> &str {
        match self.priority {
            Some(constants::PRIORITY_1) => "Priority 1: high",
            Some(constants::PRIORITY_2) => "Priority 2: medium",
            Some(constants::PRIORITY_3) => "Priority 3: low",
            _ => "Priority 4: none",
        }
    }
    pub fn custom_order(&self) -> bool {
        false
    }
    pub fn set_custom_order(&mut self, order: bool) {
        todo!();
    }
    pub fn pinned_icon(&self) -> String {
        match self.pinned {
            Some(1) => "planner-pin-tack".to_string(),
            _ => "planner-pinned".to_string(),
        }
    }
    pub fn completed(&self) -> bool {
        self.checked == Some(1)
    }
    pub fn has_due(&self) -> bool {
        self.due()
            .datetime()
            .map_or(false, |dt| dt != EMPTY_DATETIME)
    }
    pub fn has_time(&self) -> bool {
        self.due()
            .datetime()
            .map_or(false, |dt| utils::DateTime::default().has_time(&dt))
    }
    pub fn completed_date(&self) -> NaiveDateTime {
        self.completed_at
            .as_deref()
            .and_then(|s| {
                utils::DateTime::default()
                    .get_date_from_string(s.to_string())
                    .into()
            })
            .unwrap_or_else(|| EMPTY_DATETIME)
    }
    pub fn has_parent(&self) -> bool {
        self.parent_id
            .as_deref()
            .and_then(|id| Store::instance().get_item(id))
            .is_some()
    }
    pub fn has_section(&self) -> bool {
        self.section_id
            .as_deref()
            .and_then(|id| Store::instance().get_item(id))
            .is_some()
    }
    pub fn show_item(&self) -> bool {
        false
    }
    pub fn set_show_item(&mut self, v: bool) {
        todo!();
    }
    pub fn ics(&self) -> &str {
        // Services.Todoist.get_default ().get_string_member_by_object (extra_data, "ics")
        ""
    }
    pub fn added_datetime(&self) -> NaiveDateTime {
        self.added_at
            .as_deref()
            .and_then(|s| {
                utils::DateTime::default()
                    .get_date_from_string(s.to_string())
                    .into()
            })
            .unwrap_or_else(|| EMPTY_DATETIME)
    }
    pub fn updated_datetime(&self) -> NaiveDateTime {
        self.updated_at
            .as_deref()
            .and_then(|s| {
                utils::DateTime::default()
                    .get_date_from_string(s.to_string())
                    .into()
            })
            .unwrap_or_else(|| EMPTY_DATETIME)
    }
    pub fn parent(&self) -> Option<Item> {
        self.parent_id
            .as_deref()
            .and_then(|id| Store::instance().get_item(id))
    }
    pub fn project(&self) -> Option<Project> {
        self.project_id
            .as_deref()
            .and_then(|id| Store::instance().get_project(id))
    }
    pub fn section(&self) -> Option<Section> {
        self.section_id
            .as_deref()
            .and_then(|id| Store::instance().get_section(id))
    }
    // subitems
    pub fn items(&self) -> Vec<Item> {
        let mut items = Store::instance().get_subitems(self);
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }
    pub fn items_uncomplete(&self) -> Vec<Item> {
        Store::instance().get_subitems_uncomplete(self)
    }
    pub fn reminders(&self) -> Vec<Reminder> {
        Store::instance().get_reminders_by_item(self)
    }
    pub fn attachments(&self) -> Vec<Attachment> {
        Store::instance().get_attachments_by_item(self)
    }

    pub fn get_caldav_categories(&self) {}
    pub fn check_labels(&self, new_labels: HashMap<String, Label>) {
        for (key, label) in &new_labels {
            let label_id = label.id();
            if self.get_label(label_id) == None {
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
    pub fn set_section(&mut self, section: Section) {
        self.section_id = section.id;
    }
    pub fn set_project(&mut self, project: Project) {
        self.project_id = project.id;
    }
    pub fn set_parent(&mut self, parent: Item) {
        self.parent_id = parent.id;
    }

    fn add_label_if_not_exist(&self, label: Label) {
        todo!()
    }

    pub fn delete_item_label(&self, id: &str) {
        todo!()
    }
    pub fn update_local(&self) {
        Store::instance().update_item(self, "");
    }
    pub fn update(&self, update_id: &str) {
        if let Some(project) = self.project() {
            match project.source_type() {
                SourceType::LOCAL => {
                    Store::instance().update_item(self, update_id);
                }
                _ => {}
            }
        }
    }
    pub fn was_archived(&self) -> bool {
        self.parent()
            .map(|p| p.was_archived())
            .or_else(|| self.section().map(|s| s.was_archived()))
            .unwrap_or_else(|| self.project().map_or(false, |p| p.is_archived()))
    }
    fn source(&self) -> Option<Source> {
        self.project()
            .as_ref()
            .map_or(Some(Source::default()), |p| p.source())
    }
    pub fn remove_all_relative_reminders(&self) {
        self.reminders()
            .iter()
            .filter(|r| r.reminder_type() == ReminderType::RELATIVE)
            .for_each(|r| {
                r.delete();
            });
    }
    fn add_reminder_if_not_exists(&self, reminder: &Reminder) {
        todo!()
    }
    pub fn add_reminder(&self, reminder: &mut Reminder) {
        reminder.item_id = self.id.clone();
        if let Some(project) = self.project() {
            match project.source_type() {
                SourceType::LOCAL => {
                    reminder.id = Some(Util::get_default().generate_id());
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
                    Store::instance().complete_item(self);
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
        self.id.as_deref().unwrap_or_default()
    }

    fn set_id(&mut self, id: &str) {
        self.id = Some(id.into());
    }
}
