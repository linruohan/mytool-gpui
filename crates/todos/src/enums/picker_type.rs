use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum PickerType {
    PROJECTS,
    SECTIONS,
}

impl fmt::Display for PickerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
