use crate::BaseObject;
use crate::Item;
use crate::Source;
use crate::Store;
use crate::enums::{ReminderType, SourceType};
use crate::generate_accessors;
use crate::objects::{BaseTrait, DueDate, ToBool};
use crate::utils;
use chrono::Duration;
use chrono::NaiveDateTime;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
impl Deref for Reminder {
    type Target = BaseObject;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    pub base: BaseObject,
    pub item_id: String,
    pub notify_uid: i32,
    pub service: String,
    pub reminder_type: ReminderType,
    pub due: DueDate,
    pub mm_offset: i32,
    pub is_deleted: bool,
}
impl Reminder {}

impl Default for Reminder {
    fn default() -> Self {
        Self {
            base: BaseObject::default(),
            reminder_type: ReminderType::ABSOLUTE,
            ..Default::default()
        }
    }
}
impl Reminder {
    pub fn new() -> Self {
        Self::default()
    }
    generate_accessors!(item_id:Option<String>);
    generate_accessors!(notify_uid:Option<i32>);
    generate_accessors!(service:Option<String>);
    // generate_accessors!(reminder_type:Option<String>);
    pub fn reminder_type(&self) -> ReminderType {
        self.reminder_type
            .as_ref()
            .and_then(|s| serde_json::from_str::<ReminderType>(s).ok())
            .unwrap_or(ReminderType::ABSOLUTE)
    }
    pub fn set_reminder_type(&mut self, reminder_type: &ReminderType) {
        self.reminder_type = Some(reminder_type.to_string());
    }
    generate_accessors!(@due due:Option<String>);
    generate_accessors!(mm_offset:Option<i32>);
    generate_accessors!(@bool is_deleted:Option<i32>);

    pub fn item(&self) -> Option<Item> {
        self.item_id
            .as_ref()
            .and_then(|id| Store::instance().get_item(id))
    }
    pub fn relative_text(&self) -> String {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => self
                .due()
                .datetime()
                .map(|dt| utils::DateTime::default().get_relative_date_from_date(&dt))
                .unwrap_or_default(),
            ReminderType::RELATIVE => utils::Util::get_default()
                .get_reminders_mm_offset_text(self.mm_offset.unwrap_or(0))
                .to_string(),

            _ => String::new(),
        }
    }
    pub fn datetime(&self) -> Option<NaiveDateTime> {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => self.due().datetime(),
            _ => self.item()?.due().datetime().map(|dt| {
                let offset = -self.mm_offset.unwrap_or(0);
                let duration = Duration::minutes(offset as i64);
                dt + duration
            }),
        }
    }
    fn source(&self) -> Option<Source> {
        self.item()
            .and_then(|i| i.project().and_then(|p| p.source()))
    }
    pub fn delete(&self) {
        // if (item.project.source_type == SourceType.TODOIST) {
        //     loading = true;
        //     Services.Todoist.get_default ().delete.begin (this, (obj, res) => {
        //         if (Services.Todoist.get_default ().delete.end (res).status) {
        //             Services.Store.instance ().delete_reminder (this);
        //             loading = false;
        //         }
        //     });
        // } else {
        Store::instance().delete_reminder(self);
    }
    pub fn duplicate(&self) -> Reminder {
        Self {
            notify_uid: self.notify_uid,
            service: self.service.clone(),
            due: self.due.clone(),
            mm_offset: self.mm_offset,
            ..self.clone()
        }
    }
}
impl BaseTrait for Reminder {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: &str) {
        self.base.id = id.into();
    }
}
