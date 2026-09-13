use std::{fmt, str::FromStr};

use chrono::NaiveDateTime;
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
        // 最后尝试纯日期格式 (2025-02-22)，转换为当天的 00:00:00
        if let Ok(date) = chrono::NaiveDate::from_str(&self.date) {
            return Some(date.and_hms_opt(0, 0, 0).unwrap());
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

    /// 换成日历日，保留原有时分秒；没有时间则用 `00:00:00`。
    pub fn replacing_calendar_date(&self, date: chrono::NaiveDate) -> Self {
        let time = time_suffix(&self.date);
        let mut due = self.clone();
        due.date = format!("{} {time}", date.format("%Y-%m-%d"));
        due
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

    pub fn is_active_recurrence(&self) -> bool {
        self.is_recurring && self.recurrency_type != RecurrencyType::NONE
    }

    /// 完成本次重复后的下一期截止日期。
    ///
    /// 已到结束条件、无法解析日期或无法前进一步时返回 `None`，调用方应按普通完成处理。
    /// 过期多期时会连跳到不早于今天的下一期，避免完成后仍停在过去。
    pub fn next_due_after_completion(&self) -> Option<DueDate> {
        if !self.is_active_recurrence() || self.is_recurrency_end() {
            return None;
        }
        let start = self.datetime()?;
        let mut next_due = self.clone();
        if next_due.recurrency_interval < 1 {
            next_due.recurrency_interval = 1;
        }
        let helper = DateTime::default();
        let mut next_dt = helper.next_recurrency(start, next_due.clone());
        if next_dt <= start {
            return None;
        }
        let today = chrono::Local::now().naive_local().date();
        let mut hops = 0;
        while next_dt.date() < today && hops < 8000 {
            let stepped = helper.next_recurrency(next_dt, next_due.clone());
            if stepped <= next_dt {
                break;
            }
            next_dt = stepped;
            hops += 1;
        }
        if self.end_type() == RecurrencyEndType::OnDate
            && self.end_datetime().is_some_and(|end| next_dt > end)
        {
            return None;
        }
        next_due.set_datetime(next_dt);
        if self.end_type() == RecurrencyEndType::AFTER {
            next_due.recurrency_count = self.recurrency_count.saturating_sub(1);
        }
        Some(next_due)
    }
}
fn time_suffix(date: &str) -> &str {
    if let Some((_, rest)) = date.split_once(' ')
        && !rest.is_empty()
    {
        return rest;
    }
    if let Some((_, rest)) = date.split_once('T')
        && !rest.is_empty()
    {
        return rest;
    }
    "00:00:00"
}

impl fmt::Display for DueDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local, NaiveTime};

    use super::*;
    use crate::enums::RecurrencyType;

    fn daily_on(date: chrono::NaiveDate) -> DueDate {
        DueDate {
            date: date
                .and_time(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            is_recurring: true,
            recurrency_type: RecurrencyType::EveryDay,
            recurrency_interval: 1,
            recurrency_supported: true,
            ..DueDate::default()
        }
    }

    #[test]
    fn non_recurring_completes_normally() {
        let due = DueDate { date: "2026-01-01 09:00:00".into(), ..DueDate::default() };
        assert!(due.next_due_after_completion().is_none());
    }

    #[test]
    fn last_counted_occurrence_completes_normally() {
        let mut due = daily_on(Local::now().date_naive());
        due.recurrency_count = 1;
        assert!(due.next_due_after_completion().is_none());
    }

    #[test]
    fn daily_from_today_rolls_to_tomorrow() {
        let today = Local::now().date_naive();
        let next = daily_on(today).next_due_after_completion().expect("should roll");
        assert_eq!(next.datetime().unwrap().date(), today + Duration::days(1));
        assert!(next.is_recurring);
    }

    #[test]
    fn overdue_daily_skips_to_today_or_later() {
        let old = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let next = daily_on(old).next_due_after_completion().expect("should skip");
        assert!(next.datetime().unwrap().date() >= Local::now().date_naive());
    }

    #[test]
    fn after_count_decrements() {
        let mut due = daily_on(Local::now().date_naive());
        due.recurrency_count = 4;
        let next = due.next_due_after_completion().unwrap();
        assert_eq!(next.recurrency_count, 3);
    }

    #[test]
    fn replacing_calendar_date_keeps_time() {
        let due = DueDate { date: "2026-01-01 17:30:00".into(), ..DueDate::default() };
        let next =
            due.replacing_calendar_date(chrono::NaiveDate::from_ymd_opt(2026, 2, 3).unwrap());
        assert_eq!(next.date, "2026-02-03 17:30:00");
    }

    #[test]
    fn replacing_calendar_date_iso_and_date_only() {
        let iso = DueDate { date: "2026-01-01T09:15:00".into(), ..DueDate::default() };
        let next =
            iso.replacing_calendar_date(chrono::NaiveDate::from_ymd_opt(2026, 3, 4).unwrap());
        assert_eq!(next.date, "2026-03-04 09:15:00");
        let bare = DueDate { date: "2026-01-01".into(), ..DueDate::default() };
        let next =
            bare.replacing_calendar_date(chrono::NaiveDate::from_ymd_opt(2026, 3, 4).unwrap());
        assert_eq!(next.date, "2026-03-04 00:00:00");
    }
}
