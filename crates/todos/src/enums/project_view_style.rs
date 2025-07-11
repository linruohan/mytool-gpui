use std::fmt;

use strum::EnumString;
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum ProjectViewStyle {
    LIST,
    BOARD,
}
impl ProjectViewStyle {
    pub fn parse(value: &str) -> ProjectViewStyle {
        match value {
            "list" => ProjectViewStyle::LIST,
            "board" => ProjectViewStyle::BOARD,
            _ => ProjectViewStyle::LIST,
        }
    }
    pub fn to_lowercase(&self) -> String {
        match self {
            ProjectViewStyle::LIST => "list".to_string(),
            ProjectViewStyle::BOARD => "board".to_string(),
        }
    }
}
impl fmt::Display for ProjectViewStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
