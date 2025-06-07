use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
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
}
impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
