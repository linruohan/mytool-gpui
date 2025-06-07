use std::{cmp::Ordering, str::FromStr};

use crate::{Item, enums::RecurrencyType, objects::DueDate};
use anyhow::Result;
use chrono::{
    Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseError, Timelike,
};
use chrono_humanize::{Accuracy, HumanTime};
use diesel::sql_types::Json;
pub const EMPTY_DATETIME: NaiveDateTime =
    chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc();
pub struct DateTime {}
impl DateTime {
    pub fn default() -> DateTime {
        Self {}
    }
    pub fn get_todoist_datetime(&self, date: String) -> Result<NaiveDateTime, ParseError> {
        // YYYY-MM-DD
        if date.len() == 10 {
            return NaiveDateTime::parse_from_str(
                format!("{} 00:00:00", date).as_str(),
                "%Y-%m-%d %H:%M:%S",
            );
        } else {
            // YYYY-MM-DDTHH:MM:SS
            return NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S");
        }
    }

    pub fn get_relative_date_from_date(&self, datetime: &NaiveDateTime) -> String {
        let mut returned = "";
        let format_str = self.get_default_date_format_from_date(datetime).clone();
        if (self.is_today(datetime)) {
            returned = "Today";
        } else if self.is_tomorrow(datetime) {
            returned = "Tomorrow";
        } else if self.is_yesterday(datetime) {
            returned = "Yesterday";
        } else {
            returned = format_str.as_str();
        }
        if self.has_time(datetime) {
            return format!(
                "{} {}",
                returned,
                datetime.format(self.get_default_time_format())
            );
        }
        return returned.to_string();
    }

    pub fn get_relative_datetime(&self, datetime: NaiveDateTime) -> String {
        let now = Local::now().naive_local();

        let human_time = HumanTime::from(now - datetime);
        human_time.to_text_en(Accuracy::Rough, chrono_humanize::Tense::Past)
    }
    fn format_duration(&self, days: i64, suffix: &str) -> String {
        let unit = if days > 1 { "days" } else { "day" };
        format!("{} {} {}", days, unit, suffix)
    }
    pub fn days_left(&self, datetime: &NaiveDateTime, show_today: bool) -> String {
        let days = (*datetime - Local::now().naive_local()).num_days();
        match (self.is_today(datetime), days.cmp(&0)) {
            (true, _) if show_today => "Today".into(),
            (true, _) => String::new(),
            (false, Ordering::Less) => self.format_duration(-days, "ago"),
            (false, Ordering::Greater) => self.format_duration(days + 1, "left"),
            (false, Ordering::Equal) => String::new(),
        }
    }

    pub fn get_default_time_format(&self) -> &str {
        if self.is_clock_format_12h() {
            "%I:%M"
        } else {
            "%H:%M"
        }
    }

    pub fn is_clock_format_12h(&self) -> bool {
        // return Services.Settings.get_default ().settings.get_string ("clock-format").contains ("12h");
        false
    }

    pub fn is_yesterday(&self, date: &NaiveDateTime) -> bool {
        return self.is_same_day(date, &(Local::now().naive_local() - Duration::days(1)));
    }

    pub fn is_same_day(&self, dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> bool {
        return dt1.year() == dt2.year() && dt1.day() == dt2.day();
    }

    pub fn is_overdue(&self, date: &NaiveDateTime) -> bool {
        // date在今天之前，说明是过期了
        if date < &Local::now().naive_local() {
            return true;
        }
        return false;
    }

    pub fn get_calendar_icon(&self, date: &NaiveDateTime) -> &str {
        if self.is_today(date) {
            return "planner-today";
        }
        return "planner-scheduled";
    }

    pub fn parse_todoist_recurrency(duedate: DueDate, object: Json) {
        // if (object.has_member("lang") && object.get_string_member("lang") != "en") {
        //     duedate.recurrency_supported = false;
        // }
    }

    pub fn has_time(&self, datetime: &NaiveDateTime) -> bool {
        datetime == &EMPTY_DATETIME && datetime.time() != NaiveTime::default()
    }

    pub fn is_today(&self, date: &NaiveDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        return self.is_same_day(date, &Local::now().naive_local());
    }

    pub fn is_tomorrow(&self, date: &NaiveDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        return self.is_same_day(date, &(Local::now().naive_local() + Duration::days(1)));
    }

    pub fn is_next_week(&self, date: &NaiveDateTime) -> bool {
        if date == &EMPTY_DATETIME {
            return false;
        }
        return self.is_same_day(date, &(Local::now().naive_local() + Duration::days(7)));
    }

    pub fn get_date_from_string(&self, date: String) -> NaiveDateTime {
        NaiveDateTime::from_str(&date).unwrap()
    }

    pub fn recurrence_to_due(&self, recurrence: RecurrencyType, due: &mut DueDate) {
        due.is_recurring = true;
        // due.recurrency_interval = recurrence.interval;
        // due.recurrency_count = recurrence.get_count ();
        // ICal.Time until = recurrence.get_until ();
        // if (!until.is_null_time ()) {
        //     due.recurrency_end = ical_to_date_time_local (until).to_string ();
        // }

        if (due.recurrency_type == RecurrencyType::EveryWeek) {
            let mut recurrency_weeks = String::new();
            // let day_array = recurrence.get_by_day_array ();
            let mut day_array: Vec<NaiveDateTime> = Vec::new();

            if self.check_by_day("1", &day_array) {
                recurrency_weeks.push_str("7,");
            }

            if self.check_by_day("2", &day_array) {
                recurrency_weeks.push_str("1,");
            }

            if self.check_by_day("3", &day_array) {
                recurrency_weeks.push_str("2,");
            }

            if (self.check_by_day("4", &day_array)) {
                recurrency_weeks.push_str("3,");
            }

            if (self.check_by_day("5", &day_array)) {
                recurrency_weeks.push_str("4,");
            }

            if (self.check_by_day("6", &day_array)) {
                recurrency_weeks.push_str("5,");
            }

            if (self.check_by_day("7", &day_array)) {
                recurrency_weeks.push_str("6,");
            }
            let week_array: Vec<&str> = recurrency_weeks.split(",").collect();
            if (week_array.len() > 0) {
                let new_idx = recurrency_weeks.len().saturating_sub(1);
                recurrency_weeks = recurrency_weeks[0..new_idx].to_string();
            }
            due.recurrency_weeks = recurrency_weeks;
        }
    }

    fn check_by_day(&self, day: &str, day_array: &Vec<NaiveDateTime>) -> bool {
        for day1 in day_array.iter() {
            if day1.to_string() == day.to_string() {
                return true;
            }
        }
        return false;
    }

    pub fn format_date(&self, format_date: NaiveDate) -> NaiveDateTime {
        NaiveDateTime::new(format_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    }

    pub fn is_this_week(&self, date: NaiveDateTime) -> bool {
        let current_date = date.date();
        let start_of_week =
            current_date - Duration::days(current_date.weekday().num_days_from_monday() as i64);
        let end_of_week = start_of_week + Duration::days(6);

        if date >= self.format_date(start_of_week) && date <= self.format_date(end_of_week) {
            return true;
        }
        return false;
    }

    pub fn is_next_x_week(&self, date: NaiveDateTime, days: i64) -> bool {
        let current_date = Local::now().naive_local();
        let end_date = current_date + Duration::days(days);

        if date >= current_date && date <= end_date {
            return true;
        }
        return false;
    }

    pub fn is_this_month(&self, date: NaiveDateTime) -> bool {
        let current_date = Local::now().naive_local();
        return current_date.month() == date.month() && current_date.year() == date.year();
    }

    pub fn add_years(&self, datetime: NaiveDateTime, years: i32) -> NaiveDateTime {
        let date = datetime.date();
        let time = datetime.time();
        date.with_year(date.year() + years).unwrap().and_time(time)
    }
    pub fn add_months(&self, datetime: NaiveDateTime, months: u32) -> NaiveDateTime {
        let mut year = datetime.year();
        let mut month = datetime.month();
        let mut day = datetime.day();
        month += months;
        // 计算新的年和月
        while month > 12 {
            month -= 12;
            year += 1;
        }
        // 处理月份溢出问题
        let new_date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| {
            // 如果日期无效，比如2.30,则回退到月的第一天
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap()
                .pred_opt()
                .expect("add nativedatetime failed")
        });
        new_date.and_time(datetime.time())
    }

    pub fn next_recurrency(&self, datetime: NaiveDateTime, duedate: DueDate) -> NaiveDateTime {
        let returned = datetime;
        match duedate.recurrency_type {
            RecurrencyType::MINUTELY => returned + Duration::minutes(duedate.recurrency_interval),
            RecurrencyType::HOURLY => returned + Duration::hours(duedate.recurrency_interval),
            RecurrencyType::EveryDay => returned + Duration::days(duedate.recurrency_interval),
            RecurrencyType::EveryWeek => {
                if duedate.recurrency_weeks == "" {
                    return returned + Duration::days(duedate.recurrency_interval * 7);
                } else {
                    return self.next_recurrency_week(datetime, duedate, false);
                }
            }
            RecurrencyType::EveryMonth => {
                self.add_months(datetime, duedate.recurrency_interval as u32)
            }
            RecurrencyType::EveryYear => {
                self.add_years(datetime, duedate.recurrency_interval as i32)
            }
            _ => returned,
        }
    }

    pub fn get_next_day_of_week_from_recurrency_week(
        datetime: NaiveDateTime,
        duedate: DueDate,
    ) -> Option<i32> {
        let weeks: Vec<&str> = duedate.recurrency_weeks.split(",").collect();
        let day_of_week = datetime.weekday().num_days_from_monday() as i32;
        for week in &weeks {
            if let Ok(week) = week.parse::<i32>() {
                if week > day_of_week {
                    return Some(week);
                }
            } else {
                eprintln!("failed to parse week:{week}");
            }
        }

        if let Some(first_week_str) = weeks.get(0) {
            if let Ok(first_week) = first_week_str.parse::<i32>() {
                return Some(first_week);
            }
        }
        None
    }

    pub fn next_recurrency_week(
        &self,
        datetime: NaiveDateTime,
        duedate: DueDate,
        user: bool,
    ) -> NaiveDateTime {
        let weeks: Vec<&str> = duedate.recurrency_weeks.split(",").collect(); // [1, 2, 3]
        let day_of_week: i64 = datetime.weekday().num_days_from_monday() as i64; // 2
        let mut days: i64 = 0;
        let mut next_day: i64 = 0;
        let mut index = 0;
        let mut recurrency_interval = 0;

        for i in 0..weeks.len() {
            if day_of_week < weeks[i].parse::<i64>().unwrap() {
                index = i;
                break;
            }
        }
        next_day = weeks[index].parse::<i64>().unwrap();

        if (day_of_week < next_day) {
            days = next_day - day_of_week;
        } else {
            days = 7 - (day_of_week - next_day);
        }

        if (user && index == 0) {
            recurrency_interval = (duedate.recurrency_interval - 1) * 7;
        }

        return datetime + Duration::days(days) + Duration::days(recurrency_interval);
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
                weeks.push_str("Mo,");
            }

            if recurrency_weeks.contains("2") {
                weeks.push_str("Tu,");
            }

            if recurrency_weeks.contains("3") {
                weeks.push_str("We,");
            }

            if recurrency_weeks.contains("4") {
                weeks.push_str("Th,");
            }

            if recurrency_weeks.contains("5") {
                weeks.push_str("Fr,");
            }

            if recurrency_weeks.contains("6") {
                weeks.push_str("Sa,");
            }

            if recurrency_weeks.contains("7") {
                weeks.push_str("Su,");
            }

            if !weeks.is_empty() {
                weeks.pop(); // Remove the trailing comma
            }

            returned = format!("{} ({})", returned, weeks);
        }
        format!("{} {}", returned, end)
    }

    pub fn get_today_format_date(&self) -> NaiveDateTime {
        return self.get_date_only(&Local::now().naive_local());
    }

    pub fn get_date_only(&self, date: &NaiveDateTime) -> NaiveDateTime {
        NaiveDateTime::new(date.date(), NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    }

    pub fn get_default_date_format_from_date(&self, date: &NaiveDateTime) -> String {
        let year = if date.year() == Local::now().year() {
            ""
        } else {
            "%Y-"
        };
        format!("{}%m-%d %p", year)
    }

    pub fn get_todoist_datetime_format(&self, date: &NaiveDateTime) -> String {
        if (self.has_time(date)) {
            return format!(
                "{}T{}",
                date.format("%F").to_string(),
                date.format("%T").to_string()
            );
        } else {
            return date.format("%F").to_string();
        }

        return "".to_string();
    }

    pub fn has_time_from_string(&self, date: &NaiveDateTime) -> bool {
        return self.has_time(date);
    }

    pub fn get_days_of_month(index: i32, year_nav: i32) -> i32 {
        if ((index == 1)
            || (index == 3)
            || (index == 5)
            || (index == 7)
            || (index == 8)
            || (index == 10)
            || (index == 12))
        {
            return 31;
        } else {
            if (index == 2) {
                if (year_nav % 4 == 0) {
                    return 29;
                } else {
                    return 28;
                }
            } else {
                return 30;
            }
        }
    }

    pub fn get_start_of_month(&self, date: NaiveDateTime) -> NaiveDateTime {
        if date == EMPTY_DATETIME {
            let date = Local::now().naive_local();
        }
        let date1 = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
        let time1 = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        NaiveDateTime::new(date1, time1)
    }

    pub fn is_current_month(&self, date: NaiveDateTime) -> bool {
        let now = Local::now().naive_local();
        if (date.year() == now.year()) {
            if (date.month() == now.month()) {
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    // /**
    //  * Converts the given ICal.Time to a GLib.DateTime, represented in the
    //  * system timezone.
    //  *
    //  * All timezone information in the original @date is lost. However, the
    //  * {@link GLib.TimeZone} contained in the resulting DateTime is correct,
    //  * since there is a well-defined local timezone between both libical and
    //  * GLib.
    //  */
    //  pub static DateTime ical_to_date_time_local (ICal.Time date) {
    //     assert (!date.is_null_time ());
    //     let converted = ical_convert_to_local (date);
    //     int year, month, day, hour, minute, second;
    //     converted.get_date (out year, out month, out day);
    //     converted.get_time (out hour, out minute, out second);
    //     return new DateTime.local (year, month,
    //         day, hour, minute, second);
    // }

    // /** Converts the given ICal.Time to the local (or system) timezone */
    // pub static ICal.Time ical_convert_to_local (ICal.Time time) {
    //     let system_tz = ECal.util_get_system_timezone ();
    //     return time.convert_to_zone (system_tz);
    // }

    // /**
    //  * Converts two DateTimes representing a date and a time to one TimeType.
    //  *
    //  * The first contains the date; its time settings are ignored. The second
    //  * one contains the time itself; its date settings are ignored. If the time
    //  * is `null`, the resulting TimeType is of `DATE` type; if it is given, the
    //  * TimeType is of `DATE-TIME` type.
    //  *
    //  * This also accepts an optional `timezone` argument. If it is given a
    //  * timezone, the resulting TimeType will be relative to the given timezone.
    //  * If it is `null`, the resulting TimeType will be "floating" with no
    //  * timezone. If the argument is not given, it will default to the system
    //  * timezone.
    //  */
    //  pub static ICal.Time datetimes_to_icaltime (GLib.DateTime date, GLib.DateTime? time_local,
    //     ICal.Timezone? timezone = ECal.util_get_system_timezone ().copy ()) {
    //     let result = new ICal.Time.from_day_of_year (date.get_day_of_year (), date.get_year ());

    //     // Check if it's a date. If so, set is_date to true and fix the time to be sure.
    //     // If it's not a date, first thing set is_date to false.
    //     // Then, set the timezone.
    //     // Then, set the time.
    //     if (time_local == null) {
    //         // Date type: ensure that everything corresponds to a date
    //         result.set_is_date (true);
    //         // result.set_time (0, 0, 0);
    //     } else {
    //         // Includes time
    //         // Set is_date first (otherwise timezone won't change)
    //         result.set_is_date (false);

    //         // Set timezone for the time to be relative to
    //         // (doesn't affect DATE-type times)
    //         result.set_timezone (timezone);

    //         // Set the time with the updated time zone
    //         result.set_time (time_local.get_hour (), time_local.get_minute (), time_local.get_second ());
    //     }

    //     return result;
    // }

    pub fn get_markdown_format_date(&self, item: &Item) -> String {
        item.due()
            .datetime()
            .map(|dt| format!(" ({}) ", self.get_relative_date_from_date(&dt)))
            .unwrap_or_else(|| " ".to_string())
    }

    pub fn get_datetime_no_seconds(&self, datetime: NaiveDateTime) -> NaiveDateTime {
        let time = datetime.time();
        NaiveDateTime::new(
            datetime.date(),
            NaiveTime::from_hms_opt(time.hour(), time.minute(), 0).unwrap(),
        )
    }
}
