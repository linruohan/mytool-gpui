use std::fmt;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "camelCase")]
pub enum ReminderType {
    ABSOLUTE,
    RELATIVE,
}

impl fmt::Display for ReminderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
