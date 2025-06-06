use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum ObjectEventType {
    INSERT,
    UPDATE,
}
impl ObjectEventType {
    pub fn parse(value: Option<&str>) -> ObjectEventType {
        match value {
            Some("insert") => ObjectEventType::INSERT,
            Some("update") => ObjectEventType::UPDATE,
            _ => ObjectEventType::INSERT,
        }
    }
    pub fn get_label(&self) -> &str {
        match self {
            ObjectEventType::INSERT => "Task Created",
            ObjectEventType::UPDATE => "Task Updated",
        }
    }
    pub fn to_lowercase(&self) -> String {
        match self {
            ObjectEventType::INSERT => "insert".to_string(),
            ObjectEventType::UPDATE => "update".to_string(),
        }
    }
}
impl fmt::Display for ObjectEventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
