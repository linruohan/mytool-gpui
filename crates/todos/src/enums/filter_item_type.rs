use std::{
    default,
    fmt::{self, Display},
};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, EnumString, Default, Eq, Hash, Deserialize, Serialize)]
#[strum(serialize_all = "camelCase")]
pub enum FilterItemType {
    #[default]
    PRIORITY = 0,
    LABEL = 1,
    DueDate = 2,
    SECTION = 3,
}
impl FilterItemType {
    pub fn get_title(&self) -> &str {
        match self {
            FilterItemType::PRIORITY => "Priority",
            FilterItemType::LABEL => "Label",
            FilterItemType::DueDate => "Due Date",
            FilterItemType::SECTION => "Section",
        }
    }

    pub fn get_icon(&self) -> &str {
        match self {
            FilterItemType::PRIORITY => "flag-outline-thick-symbolic",
            FilterItemType::LABEL => "tag-outline-symbolic",
            FilterItemType::DueDate => "month-symbolic",
            FilterItemType::SECTION => "arrow3-right-symbolic",
        }
    }

    pub fn to_lowercase(&self) -> String {
        match self {
            FilterItemType::PRIORITY => "priority".to_string(),
            FilterItemType::LABEL => "label".to_string(),
            FilterItemType::DueDate => "duedate".to_string(),
            FilterItemType::SECTION => "section".to_string(),
        }
    }
}

impl fmt::Display for FilterItemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
