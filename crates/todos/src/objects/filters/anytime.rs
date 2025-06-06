use std::{any::Any, collections::HashMap};

use uuid::Uuid;

use super::FilterItem;
use crate::{BaseObject, BaseTrait};
pub struct Anytime {
    pub base: BaseObject,
}

impl Default for Anytime {
    fn default() -> Self {
        Anytime {
            base: BaseObject::new(
                "Anytime".to_string(),
                format!("{};{};{}", "anytime", "filters", "no date"),
                "grid-large-symbolic".to_string(),
                "anytime-view".to_string(),
            ),
        }
    }
}
