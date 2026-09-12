use serde::{Deserialize, Serialize};
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Debug, PartialEq, Eq, Clone, Deserialize)]
pub enum RecurrencyType {
    MINUTELY,
    HOURLY,
    EveryDay,
    EveryWeek,
    EveryMonth,
    EveryYear,
    NONE,
}
impl RecurrencyType {
    pub fn to_friendly_string(&self, interval: i32) -> String {
        let interval = interval.max(1);
        match self {
            RecurrencyType::NONE => "不重复".to_owned(),
            RecurrencyType::MINUTELY => format!("每{interval}分钟"),
            RecurrencyType::HOURLY => format!("每{interval}小时"),
            RecurrencyType::EveryDay => format!("每{interval}天"),
            RecurrencyType::EveryWeek => format!("每{interval}周"),
            RecurrencyType::EveryMonth => format!("每{interval}月"),
            RecurrencyType::EveryYear => format!("每{interval}年"),
        }
    }
}
