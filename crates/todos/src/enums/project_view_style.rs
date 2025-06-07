use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum ProjectViewStyle {
    PROGRESS,
    EMOJI,
}
impl ProjectViewStyle {
    pub fn parse(value: Option<&str>) -> ProjectViewStyle {
        match value {
            Some("progress") => ProjectViewStyle::PROGRESS,
            Some("emoji") => ProjectViewStyle::EMOJI,
            _ => ProjectViewStyle::PROGRESS,
        }
    }
}
impl fmt::Display for ProjectViewStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
