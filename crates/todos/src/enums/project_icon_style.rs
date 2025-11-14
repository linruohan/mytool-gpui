use std::fmt;

use strum::EnumString;
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum ProjectIconStyle {
    PROGRESS,
    EMOJI,
}
impl ProjectIconStyle {
    pub fn parse(value: &str) -> ProjectIconStyle {
        match value {
            "process" => ProjectIconStyle::PROGRESS,
            "emoji" => ProjectIconStyle::EMOJI,
            _ => ProjectIconStyle::PROGRESS,
        }
    }

    pub fn to_lowercase(&self) -> &str {
        match self {
            ProjectIconStyle::PROGRESS => "process",
            ProjectIconStyle::EMOJI => "emoji",
            _ => "process",
        }
    }
}
impl fmt::Display for ProjectIconStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
