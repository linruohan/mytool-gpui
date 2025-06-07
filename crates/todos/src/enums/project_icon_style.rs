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
}
impl fmt::Display for ProjectIconStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
