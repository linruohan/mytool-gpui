use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum PickerType {
    PROJECTS,
    SECTIONS,
}
impl PickerType {
    pub fn to_lowercase(&self) -> String {
        match self {
            PickerType::PROJECTS => "projects".to_string(),
            PickerType::SECTIONS => "sections".to_string(),
        }
    }
}

impl fmt::Display for PickerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
