use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum ProjectIconStyle {
    LIST,
    BOARD,
}
impl ProjectIconStyle {
    pub fn parse(value: Option<&str>) -> ProjectIconStyle {
        match value {
            Some("list") => ProjectIconStyle::LIST,
            Some("board") => ProjectIconStyle::BOARD,
            _ => ProjectIconStyle::LIST,
        }
    }
    pub fn to_lowercase(&self) -> String {
        match self {
            ProjectIconStyle::LIST => "list".to_string(),
            ProjectIconStyle::BOARD => "board".to_string(),
        }
    }
}
impl fmt::Display for ProjectIconStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
