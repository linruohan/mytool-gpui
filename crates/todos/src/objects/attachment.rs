use std::fmt;

use uuid::Uuid;

use crate::Store;

use super::Item;
use serde::{Deserialize, Serialize};
pub struct Attachment {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub file_type: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<String>,
    pub file_path: Option<String>,
}

impl Attachment {
    pub fn new(
        file_type: Option<String>,
        file_name: Option<String>,
        file_size: Option<String>,
        file_path: Option<String>,
    ) -> Attachment {
        Self {
            id: Some(Uuid::new_v4().to_string()),
            item_id: Some("".to_string()),
            file_type,
            file_name,
            file_size,
            file_path,
        }
    }
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }
    pub fn delete(&self) {
        Store::instance().delete_attachment(self);
    }
    pub fn item(&self) -> Item {
        Store::instance().get_item(self.id()).unwrap()
    }
    pub fn set_item(&mut self, new_item_id: String) {
        self.item_id = Some(new_item_id);
    }

    pub fn duplicate(&self) -> Attachment {
        Attachment::new(
            self.file_type.clone(),
            self.file_name.clone(),
            self.file_size.clone(),
            self.file_path.clone(),
        )
    }
}
impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "_________________________________\nID: {}\nITEM ID: {}\nFILE TYPE: {}\nFILE NAME: {}\nFILE SIZE: {}\nFILE PATH: {}\n---------------------------------",
            self.id.clone().unwrap(),
            self.item_id.clone().unwrap(),
            self.file_type.clone().unwrap(),
            self.file_name.clone().unwrap(),
            self.file_size.clone().unwrap(),
            self.file_path.clone().unwrap(),
        )
    }
}
