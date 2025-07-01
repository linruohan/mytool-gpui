use std::fmt;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString, Eq, Deserialize, Serialize)]
#[strum(serialize_all = "kebab-case")] // 自动处理连字符格式
pub enum SourceType {
    NONE,
    LOCAL,
    TODOIST,
    GoogleTasks,
    #[strum(serialize = "caldav")] // 显式指定特殊格式
    CALDAV,
}
impl SourceType {
    pub fn parse(value: Option<&str>) -> SourceType {
        value
            .and_then(|s| s.parse().ok())
            .unwrap_or(SourceType::NONE)
    }
    pub fn to_lowercase(&self) -> String {
        match self {
            SourceType::NONE => "none".to_string(),
            SourceType::LOCAL => "local".to_string(),
            SourceType::TODOIST => "todoist".to_string(),
            SourceType::GoogleTasks => "google-tasks".to_string(),
            SourceType::CALDAV => "caldav".to_string(),
        }
    }
}
impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
