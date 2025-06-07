use serde::{Deserialize, Serialize};

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
        match self {
            RecurrencyType::NONE => "Don't Repeat".to_owned(),
            _ => {
                let s = serde_json::to_string(self)
                    .unwrap()
                    .to_lowercase()
                    .replace("ly", "")
                    .replace("every", "");
                if interval == 0 {
                    return format!("Every {}", s);
                } else {
                    return format!("Every {}  Every {} {}s", s, interval, s);
                }
            }
        }
    }
}
