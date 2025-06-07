use crate::enums::{RecurrencyEndType, RecurrencyType};
use crate::utils::DateTime;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Debug, PartialEq, Eq, Serialize, Clone, Deserialize)]
pub struct DueDate {
    pub date: String,
    pub timezone: String,
    pub recurrency_weeks: String,
    pub is_recurring: bool,
    pub recurrency_type: RecurrencyType,
    pub recurrency_interval: i64,
    pub recurrency_count: i64,
    pub recurrency_end: String,
    pub recurrency_supported: bool,
}
impl DueDate {
    pub fn default() -> DueDate {
        Self {
            date: "".to_string(),
            timezone: "".to_string(),
            recurrency_weeks: "".to_string(),
            is_recurring: false,
            recurrency_type: RecurrencyType::NONE,
            recurrency_interval: 0,
            recurrency_count: 0,
            recurrency_end: "".to_string(),
            recurrency_supported: false,
        }
    }
    pub fn datetime(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::from_str(&self.date).ok()
    }
    pub fn set_datetime(&mut self, value: NaiveDateTime) {
        self.date = value.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    pub fn end_datetime(&self) -> NaiveDateTime {
        NaiveDateTime::from_str(&self.recurrency_end).unwrap()
    }
    pub fn has_weeks(&self) -> bool {
        self.recurrency_weeks != ""
    }
    pub fn end_type(&self) -> RecurrencyEndType {
        if self.recurrency_end != "" {
            return RecurrencyEndType::OnDate;
        }
        if self.recurrency_count > 0 {
            return RecurrencyEndType::AFTER;
        }
        return RecurrencyEndType::NEVER;
    }
    pub fn is_recurrency_end(&self) -> bool {
        match self.end_type() {
            RecurrencyEndType::AFTER => self.recurrency_count - 1 <= 0,
            RecurrencyEndType::OnDate => {
                let next_recurrency: NaiveDateTime = self
                    .datetime()
                    .map(|dt| DateTime::default().next_recurrency(dt, self.clone()))
                    .unwrap_or_default();
                return next_recurrency > self.end_datetime();
            }
            _ => false,
        }
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn is_recurrency_equal(&self, duedate: DueDate) -> bool {
        return self.recurrency_type == duedate.recurrency_type
            && self.recurrency_interval == duedate.recurrency_interval
            && self.recurrency_weeks == duedate.recurrency_weeks
            && self.recurrency_count == duedate.recurrency_count
            && self.recurrency_end == duedate.recurrency_end
            && self.is_recurring == duedate.is_recurring;
    }

    pub fn to_friendly_string(&self) -> String {
        return self
            .recurrency_type
            .to_friendly_string(self.recurrency_interval as i32);
    }
    pub fn reset(&mut self) {
        self.date = "".to_string();
        self.timezone = "".to_string();
        self.recurrency_weeks = "".to_string();
        self.is_recurring = false;
        self.recurrency_type = RecurrencyType::NONE;
        self.recurrency_end = "".to_string();
    }
    pub fn duplicate(&self) -> DueDate {
        let mut new_due = DueDate::default();
        new_due.date = self.date.clone();
        new_due.timezone = self.timezone.clone();
        new_due.recurrency_weeks = self.recurrency_weeks.clone();
        new_due.is_recurring = self.is_recurring;
        new_due.recurrency_type = self.recurrency_type.clone();
        new_due.recurrency_interval = self.recurrency_interval;
        new_due.recurrency_count = self.recurrency_count;
        new_due.recurrency_end = self.recurrency_end.clone();
        new_due.recurrency_supported = self.recurrency_supported;
        new_due
    }
}
