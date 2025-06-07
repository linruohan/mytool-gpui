use std::collections::HashMap;

use super::FilterItem;
use uuid::Uuid;

use crate::{BaseObject, objects::BaseTrait};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllItems {
    pub base: BaseObject,
}

impl Default for AllItems {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "All Tasks".to_string(),
                format!("{};{}", "all tasks", "all"),
                "check-round-outline-symbolic".to_string(),
                "all-items-view".to_string(),
            ),
        }
    }
}
