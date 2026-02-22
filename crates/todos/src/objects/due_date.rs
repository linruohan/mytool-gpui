use std::{fmt, str::FromStr};

use chrono::NaiveDateTime;
use sea_orm::Iden;
use serde::{Deserialize, Serialize};

use crate::{
    enums::{RecurrencyEndType, RecurrencyType},
    utils::DateTime,
};

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

impl Default for DueDate {
    fn default() -> Self {
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
}

impl DueDate {
    pub fn datetime(&self) -> Option<NaiveDateTime> {
        // 支持两种格式：带T的ISO格式和不带T的格式
        if self.date.is_empty() {
            return None;
        }
        // 先尝试标准ISO格式 (2025-02-22T17:30:00)
        if let Ok(dt) = NaiveDateTime::from_str(&self.date) {
            return Some(dt);
        }
        // 再尝试带空格的格式 (2025-02-22 17:30:00)
        if let Ok(dt) = NaiveDateTime::parse_from_str(&self.date, "%Y-%m-%d %H:%M:%S") {
            return Some(dt);
        }
        None
    }

    pub fn set_datetime(&mut self, value: NaiveDateTime) {
        self.date = value.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn end_datetime(&self) -> Option<NaiveDateTime> {
        self.recurrency_end.parse().ok()
    }

    pub fn has_weeks(&self) -> bool {
        !self.recurrency_weeks.is_empty()
    }

    pub fn end_type(&self) -> RecurrencyEndType {
        if !self.recurrency_end.is_empty() {
            return RecurrencyEndType::OnDate;
        }
        if self.recurrency_count > 0 {
            return RecurrencyEndType::AFTER;
        }
        RecurrencyEndType::NEVER
    }

    pub fn is_recurrency_end(&self) -> bool {
        match self.end_type() {
            RecurrencyEndType::AFTER => self.recurrency_count - 1 <= 0,
            RecurrencyEndType::OnDate => self
                .datetime()
                .map(|dt| DateTime::default().next_recurrency(dt, self.clone()))
                .is_some_and(|next| next > self.end_datetime().unwrap_or_default()),
            _ => false,
        }
    }

    pub fn is_recurrency_equal(&self, date: DueDate) -> bool {
        self.recurrency_type == date.recurrency_type
            && self.recurrency_interval == date.recurrency_interval
            && self.recurrency_weeks == date.recurrency_weeks
            && self.recurrency_count == date.recurrency_count
            && self.recurrency_end == date.recurrency_end
            && self.is_recurring == date.is_recurring
    }

    pub fn to_friendly_string(&self) -> String {
        self.recurrency_type.to_friendly_string(self.recurrency_interval as i32)
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
        DueDate {
            date: self.date.clone(),
            timezone: self.timezone.clone(),
            recurrency_weeks: self.recurrency_weeks.clone(),
            is_recurring: self.is_recurring,
            recurrency_type: self.recurrency_type.clone(),
            recurrency_interval: self.recurrency_interval,
            recurrency_count: self.recurrency_count,
            recurrency_end: self.recurrency_end.clone(),
            recurrency_supported: self.recurrency_supported,
        }
    }
}
impl fmt::Display for DueDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}
