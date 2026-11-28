use std::cmp::Ordering;

use jiff::{
    Span,
    civil::{Date, DateTime as CivilDateTime, Time},
};
use serde_json::Value;

use crate::{
    enums::RecurrencyType,
    objects::DueDate,
    utils::{now_local, parse_datetime, today_local},
};

/// 空日期时间哨兵值：1970-01-01 00:00:00
pub const EMPTY_DATETIME: CivilDateTime = CivilDateTime::constant(1970, 1, 1, 0, 0, 0, 0);

#[derive(Default)]
pub struct DateTime {}

impl DateTime {
    /// 解析 Todoist 风格的日期字符串
    pub fn get_todoist_datetime(&self, date: String) -> Result<CivilDateTime, jiff::Error> {
        // YYYY-MM-DD
        if date.len() == 10 {
            CivilDateTime::strptime("%Y-%m-%d %H:%M:%S", format!("{date} 00:00:00"))
        } else {
            // YYYY-MM-DDTHH:MM:SS
            CivilDateTime::strptime("%Y-%m-%d %H:%M:%S", &date)
        }
    }

    /// 相对日期文案：今天 / 明天 / 昨天 / M月D日
    pub fn get_relative_date_from_date(&self, datetime: &CivilDateTime) -> String {
        let now = now_local();
        let format_str =
            if datetime.year() == now.year() { "%m月%d日" } else { "%Y年%m月%d日" };
        let returned = if datetime != &EMPTY_DATETIME && self.is_same_day(datetime, &now) {
            "今天".to_string()
        } else if datetime != &EMPTY_DATETIME
            && self.is_same_day(datetime, &(now.checked_add(Span::new().days(1)).unwrap_or(now)))
        {
            "明天".to_string()
        } else if datetime != &EMPTY_DATETIME
            && self.is_same_day(datetime, &(now.checked_sub(Span::new().days(1)).unwrap_or(now)))
        {
            "昨天".to_string()
        } else {
            datetime.strftime(format_str).to_string()
        };
        if self.has_time(datetime) {
            format!("{} {}", returned, datetime.strftime(self.get_default_time_format()))
        } else {
            returned
        }
    }

    /// 相对时间文案：刚刚 / N 分钟前 / N 小时前 / N 天前 ...
    pub fn get_relative_datetime(&self, datetime: CivilDateTime) -> String {
        let now = now_local();
        // 与 chrono 的 num_seconds 一致：截断到整秒
        let total_seconds = now.duration_since(datetime).as_secs();

        if total_seconds < 60 {
            return "刚刚".to_string();
        }

        let total_minutes = total_seconds / 60;
        if total_minutes < 60 {
            return format!("{total_minutes} 分钟前");
        }

        let total_hours = total_minutes / 60;
        if total_hours < 24 {
            return format!("{total_hours} 小时前");
        }

        let total_days = total_hours / 24;
        if total_days < 30 {
            return format!("{total_days} 天前");
        }

        let total_months = total_days / 30;
        if total_months < 12 {
            return format!("{total_months} 个月前");
        }

        let total_years = total_months / 12;
        format!("{total_years} 年前")
    }

    fn format_duration(&self, days: i64, suffix: &str) -> String {
        format!("{days}天{suffix}")
    }

    /// 距离目标日期还有几天（负数表示已过）
    pub fn days_left(&self, datetime: &CivilDateTime, show_today: bool) -> String {
        let days = datetime.duration_since(now_local()).as_secs() / 86_400;
        match (self.is_today(datetime), days.cmp(&0)) {
            (true, _) if show_today => "今天".into(),
            (true, _) => String::new(),
            (false, Ordering::Less) => self.format_duration(-days, "前"),
            (false, Ordering::Greater) => self.format_duration(days + 1, "后"),
            (false, Ordering::Equal) => String::new(),
        }
    }

    pub fn get_default_time_format(&self) -> &str {
        if self.is_clock_format_12h() { "%I:%M" } else { "%H:%M" }
    }

    pub fn is_clock_format_12h(&self) -> bool {
        // return Services.Settings.get_default ().settings.get_string ("clock-format").contains
        // ("12h");
        false
    }

    pub fn is_yesterday(&self, date: &CivilDateTime) -> bool {
        let yesterday = today_local().yesterday().unwrap_or_else(|_| today_local());
        self.is_same_day(date, &yesterday.to_datetime(Time::midnight()))
    }

    pub fn is_same_day(&self, dt1: &CivilDateTime, dt2: &CivilDateTime) -> bool {
        dt1.date() == dt2.date()
    }

    pub fn is_overdue(&self, date: &CivilDateTime) -> bool {
        // date在今天之前，说明是过期了
        date < &now_local()
    }

    pub fn get_calendar_icon(&self, date: &CivilDateTime) -> &str {
        if self.is_today(date) {
            return "planner-today";
        }
        "planner-scheduled"
    }

    pub fn parse_todoist_recurrency(_duedate: DueDate, _object: Value) {}

    pub fn has_time(&self, datetime: &CivilDateTime) -> bool {
        datetime == &EMPTY_DATETIME && datetime.time() != Time::midnight()
    }

    pub fn is_today(&self, date: &CivilDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        self.is_same_day(date, &now_local())
    }

    pub fn is_tomorrow(&self, date: &CivilDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        let tomorrow = today_local().tomorrow().unwrap_or_else(|_| today_local());
        self.is_same_day(date, &tomorrow.to_datetime(Time::midnight()))
    }

    pub fn is_next_week(&self, date: &CivilDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        let today = today_local();
        let next_week = today.checked_add(Span::new().days(7)).unwrap_or(today);
        self.is_same_day(date, &next_week.to_datetime(Time::midnight()))
    }

    pub fn get_date_from_string(&self, date: &str) -> CivilDateTime {
        parse_datetime(date).unwrap_or_default()
    }

    pub fn recurrence_to_due(&self, _recurrence: RecurrencyType, due: &mut DueDate) {
        due.is_recurring = true;

        if due.recurrency_type == RecurrencyType::EveryWeek {
            let mut recurrency_weeks = String::new();
            let day_array: Vec<CivilDateTime> = Vec::new();

            if self.check_by_day("1", &day_array) {
                recurrency_weeks.push_str("7,");
            }

            if self.check_by_day("2", &day_array) {
                recurrency_weeks.push_str("1,");
            }

            if self.check_by_day("3", &day_array) {
                recurrency_weeks.push_str("2,");
            }

            if self.check_by_day("4", &day_array) {
                recurrency_weeks.push_str("3,");
            }

            if self.check_by_day("5", &day_array) {
                recurrency_weeks.push_str("4,");
            }

            if self.check_by_day("6", &day_array) {
                recurrency_weeks.push_str("5,");
            }

            if self.check_by_day("7", &day_array) {
                recurrency_weeks.push_str("6,");
            }
            let week_array: Vec<&str> = recurrency_weeks.split(",").collect();
            if !week_array.is_empty() {
                let new_idx = recurrency_weeks.len().saturating_sub(1);
                recurrency_weeks = recurrency_weeks[0..new_idx].to_string();
            }
            due.recurrency_weeks = recurrency_weeks;
        }
    }

    fn check_by_day(&self, day: &str, day_array: &[CivilDateTime]) -> bool {
        for day1 in day_array.iter() {
            if day1.to_string() == day {
                return true;
            }
        }
        false
    }

    pub fn format_date(&self, format_date: Date) -> CivilDateTime {
        format_date.to_datetime(Time::midnight())
    }

    pub fn is_this_week(&self, date: CivilDateTime) -> bool {
        let current_date = date.date();
        // 周一为一周起点
        let monday_offset = current_date.weekday().to_monday_zero_offset() as i64;
        let start_of_week =
            current_date.checked_sub(Span::new().days(monday_offset)).unwrap_or(current_date);
        let end_of_week = start_of_week.checked_add(Span::new().days(6)).unwrap_or(start_of_week);

        date >= self.format_date(start_of_week) && date <= self.format_date(end_of_week)
    }

    pub fn is_next_x_week(&self, date: CivilDateTime, days: i64) -> bool {
        let current_date = now_local();
        let end_date = current_date.checked_add(Span::new().days(days)).unwrap_or(current_date);

        date >= current_date && date <= end_date
    }

    pub fn is_this_month(&self, date: CivilDateTime) -> bool {
        let current_date = now_local();
        current_date.month() == date.month() && current_date.year() == date.year()
    }

    pub fn add_years(&self, datetime: CivilDateTime, years: i32) -> CivilDateTime {
        // jiff 默认 constrain：2 月 29 日加一年会落到 2 月 28 日
        datetime.checked_add(Span::new().years(years as i64)).unwrap_or(datetime)
    }

    pub fn add_months(&self, datetime: CivilDateTime, months: u32) -> CivilDateTime {
        datetime.checked_add(Span::new().months(months as i64)).unwrap_or(datetime)
    }

    pub fn next_recurrency(&self, datetime: CivilDateTime, duedate: DueDate) -> CivilDateTime {
        let returned = datetime;
        match duedate.recurrency_type {
            RecurrencyType::MINUTELY => returned
                .checked_add(Span::new().minutes(duedate.recurrency_interval))
                .unwrap_or(returned),
            RecurrencyType::HOURLY => returned
                .checked_add(Span::new().hours(duedate.recurrency_interval))
                .unwrap_or(returned),
            RecurrencyType::EveryDay => returned
                .checked_add(Span::new().days(duedate.recurrency_interval))
                .unwrap_or(returned),
            RecurrencyType::EveryWeek => {
                if duedate.recurrency_weeks.is_empty() {
                    returned
                        .checked_add(Span::new().days(duedate.recurrency_interval * 7))
                        .unwrap_or(returned)
                } else {
                    self.next_recurrency_week(datetime, duedate, false)
                }
            },
            RecurrencyType::EveryMonth => {
                self.add_months(datetime, duedate.recurrency_interval as u32)
            },
            RecurrencyType::EveryYear => {
                self.add_years(datetime, duedate.recurrency_interval as i32)
            },
            _ => returned,
        }
    }

    pub fn get_next_day_of_week_from_recurrency_week(
        datetime: CivilDateTime,
        duedate: DueDate,
    ) -> Option<i32> {
        let weeks: Vec<&str> = duedate.recurrency_weeks.split(",").collect();
        let day_of_week = datetime.weekday().to_monday_zero_offset() as i32;
        for week in &weeks {
            if let Ok(week) = week.parse::<i32>() {
                if week > day_of_week {
                    return Some(week);
                }
            } else {
                eprintln!("failed to parse week:{week}");
            }
        }

        if let Some(first_week_str) = weeks.first()
            && let Ok(first_week) = first_week_str.parse::<i32>()
        {
            return Some(first_week);
        }
        None
    }

    pub fn next_recurrency_week(
        &self,
        datetime: CivilDateTime,
        duedate: DueDate,
        user: bool,
    ) -> CivilDateTime {
        let weeks: Vec<&str> = duedate.recurrency_weeks.split(",").collect(); // [1, 2, 3]
        let day_of_week: i64 = datetime.weekday().to_monday_zero_offset() as i64; // 2
        let mut index = 0;
        let mut recurrency_interval = 0;

        for (i, week_str) in weeks.iter().enumerate() {
            if day_of_week < week_str.parse::<i64>().unwrap() {
                index = i;
                break;
            }
        }
        let next_day: i64 = weeks[index].parse::<i64>().unwrap();

        let days: i64 = if day_of_week < next_day {
            next_day - day_of_week
        } else {
            7 - (day_of_week - next_day)
        };

        if user && index == 0 {
            recurrency_interval = (duedate.recurrency_interval - 1) * 7;
        }

        datetime.checked_add(Span::new().days(days + recurrency_interval)).unwrap_or(datetime)
    }

    pub fn get_recurrency_weeks(
        recurrency_type: RecurrencyType,
        recurrency_interval: i32,
        recurrency_weeks: String,
        end: String,
    ) -> String {
        let mut returned = recurrency_type.to_friendly_string(recurrency_interval);

        if recurrency_type == RecurrencyType::EveryWeek && !recurrency_weeks.is_empty() {
            let mut weeks = String::new();

            if recurrency_weeks.contains("1") {
                weeks.push_str("一,");
            }

            if recurrency_weeks.contains("2") {
                weeks.push_str("二,");
            }

            if recurrency_weeks.contains("3") {
                weeks.push_str("三,");
            }

            if recurrency_weeks.contains("4") {
                weeks.push_str("四,");
            }

            if recurrency_weeks.contains("5") {
                weeks.push_str("五,");
            }

            if recurrency_weeks.contains("6") {
                weeks.push_str("六,");
            }

            if recurrency_weeks.contains("7") {
                weeks.push_str("日,");
            }

            if !weeks.is_empty() {
                weeks.pop(); // Remove the trailing comma
            }

            returned = format!("{returned} ({weeks})");
        }
        format!("{returned} {end}")
    }

    pub fn get_today_format_date(&self) -> CivilDateTime {
        self.get_date_only(&now_local())
    }

    pub fn get_date_only(&self, date: &CivilDateTime) -> CivilDateTime {
        date.date().to_datetime(Time::midnight())
    }

    pub fn get_default_date_format_from_date(&self, date: &CivilDateTime) -> String {
        if date.year() == today_local().year() {
            "%m月%d日".to_string()
        } else {
            "%Y年%m月%d日".to_string()
        }
    }

    pub fn get_todoist_datetime_format(&self, date: &CivilDateTime) -> String {
        if self.has_time(date) {
            format!("{}T{}", date.strftime("%F"), date.strftime("%T"))
        } else {
            date.strftime("%F").to_string()
        }
    }

    pub fn has_time_from_string(&self, date: &CivilDateTime) -> bool {
        self.has_time(date)
    }

    pub fn get_days_of_month(index: i32, year_nav: i32) -> i32 {
        if (index == 1)
            || (index == 3)
            || (index == 5)
            || (index == 7)
            || (index == 8)
            || (index == 10)
            || (index == 12)
        {
            31
        } else if index == 2 {
            if year_nav % 4 == 0 { 29 } else { 28 }
        } else {
            30
        }
    }

    pub fn get_start_of_month(&self, date: CivilDateTime) -> CivilDateTime {
        date.date().first_of_month().to_datetime(Time::midnight())
    }

    pub fn is_current_month(&self, date: CivilDateTime) -> bool {
        let now = now_local();
        if date.year() == now.year() { date.month() == now.month() } else { false }
    }

    pub fn get_datetime_no_seconds(&self, datetime: CivilDateTime) -> CivilDateTime {
        let time = Time::new(datetime.hour(), datetime.minute(), 0, 0).unwrap_or(datetime.time());
        CivilDateTime::from_parts(datetime.date(), time)
    }
}
