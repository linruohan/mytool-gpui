use chrono::{NaiveDateTime, Timelike};

use crate::{
    enums::{ObjectEventType, object_event_key_type::ObjectEventKeyType},
    objects::DueDate,
    utils::DateTime,
};

pub struct ObjectEvent {
    pub id: i64,
    pub event_type: ObjectEventType,
    pub event_date: String,
    pub object_id: String,
    pub object_type: String,
    pub object_key: ObjectEventKeyType,
    pub object_old_value: String,
    pub object_new_value: String,
    pub parent_item_id: String,
    pub parent_project_id: String,
}

impl ObjectEvent {
    pub fn icon_name(&self) -> &str {
        if self.event_type == ObjectEventType::INSERT {
            "plus-large-symbolic"
        } else if self.event_type == ObjectEventType::UPDATE {
            match self.object_key {
                ObjectEventKeyType::CONTENT => "edit-symbolic",
                ObjectEventKeyType::DESCRIPTION => "paper-symbolic",
                ObjectEventKeyType::DUE => "month-symbolic",
                ObjectEventKeyType::PRIORITY => "flag-outline-thick-symbolic",
                ObjectEventKeyType::LABELS => "tag-outline-symbolic",
                ObjectEventKeyType::PINNED => "pin-symbolic",
                ObjectEventKeyType::CHECKED => "check-round-outline-symbolic",
                ObjectEventKeyType::SECTION | ObjectEventKeyType::PROJECT => {
                    "arrow3-right-symbolic"
                },
            }
        } else {
            "plus-large-symbolic"
        }
    }

    pub fn datetime(&self) -> NaiveDateTime {
        DateTime::default().get_date_from_string(&self.event_date)
    }

    pub fn date(&self) -> NaiveDateTime {
        DateTime::default().format_date(self.datetime().date())
    }

    pub fn time(&self) -> String {
        if DateTime::default().is_clock_format_12h() {
            self.datetime().time().hour12().1.to_string()
        } else {
            self.datetime().time().hour().to_string()
        }
    }

    pub fn get_due_value(&self, value: String) -> Option<DueDate> {
        serde_json::from_str(value.as_str()).ok()
    }

    pub fn get_labels_value(&self, value: String) -> String {
        let labels: Vec<String> = serde_json::from_str(value.as_str()).unwrap_or_default();
        if labels.is_empty() {
            return String::new();
        }
        labels.join(", ")
    }
}
