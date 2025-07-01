use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::BaseObject;
use crate::enums::SourceType;
use crate::objects::BaseTrait;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Source {
    pub base: BaseObject,
    pub source_type: String,
    pub display_name: Option<String>,
    pub added_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_visible: Option<i32>,
    pub child_order: Option<i32>,
    pub sync_server: Option<i32>,
    pub last_sync: Option<String>,
    pub data: Option<String>,
}
impl Deref for Source {
    type Target = BaseObject;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl Source {
    pub fn default() -> Source {
        Self {
            base: BaseObject::default(),
            source_type: "".to_string(),
            display_name: todo!(),
            added_at: todo!(),
            updated_at: todo!(),
            is_visible: todo!(),
            child_order: todo!(),
            sync_server: todo!(),
            last_sync: todo!(),
            data: todo!(),
        }
    }
    pub fn source_type(&self) -> SourceType {
        SourceType::parse(Some(&self.source_type))
    }
    pub fn header_text(&self) -> String {
        self.display_name.clone().unwrap_or_default()
    }
}

impl BaseTrait for Source {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: &str) {
        self.base.id = id.into();
    }
}
