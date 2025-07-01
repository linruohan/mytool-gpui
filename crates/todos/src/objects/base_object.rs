use crate::enums::ObjectType;
use crate::filters::FilterItem;
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::collections::HashMap;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct BaseObject {
    pub id: String,
    pub name: String,
    pub keywords: String,
    pub icon_name: String,
    pub view_id: String,
    pub update_timeout_id: u32,
    pub filters: HashMap<String, FilterItem>,
}
impl BaseObject {
    pub fn new(name: String, keywords: String, icon_name: String, view_id: String) -> BaseObject {
        Self {
            id: String::from(""),
            name,
            keywords,
            icon_name,
            view_id,
            update_timeout_id: 0,
            filters: HashMap::new(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, value: impl Into<String>) {
        self.name = value.into();
    }
    pub fn keywords(&self) -> &str {
        &self.keywords
    }
    pub fn set_keywords(&mut self, keywords: impl Into<String>) {
        self.keywords = keywords.into();
    }
    pub fn icon_name(&self) -> &str {
        &self.icon_name
    }

    pub fn set_icon_name(&mut self, icon_name: impl Into<String>) {
        self.icon_name = icon_name.into();
    }
    pub fn view_id(&self) -> &str {
        &self.view_id
    }

    pub fn set_view_id(&mut self, view_id: impl Into<String>) {
        self.view_id = view_id.into();
    }
    pub fn update_timeout_id(&self) -> u32 {
        self.update_timeout_id
    }

    pub fn set_update_timeout_id(&mut self, timeout_id: u32) {
        self.update_timeout_id = timeout_id;
    }
    pub fn loading(&self) -> bool {
        false
    }
    pub fn loading_change(&self) {}
    pub fn sensitive(&self) -> bool {
        false
    }
    pub fn sensitive_change(&self) {}
    pub fn get_filter(&self, id: String) -> FilterItem {
        if let Some(filter) = self.filters.get(&id) {
            filter.clone()
        } else {
            FilterItem::default()
        }
    }
    pub fn add_filter(&mut self, filter: FilterItem) {
        self.filters.entry(filter.id().clone()).or_insert(filter);
    }
    pub fn update_filter(&mut self, update_filter: FilterItem) {
        if let Some(filter) = self.filters.get_mut(&update_filter.id().clone()) {
            *filter = update_filter;
        }
    }
    pub fn remove_filter(&mut self, filter: FilterItem) {
        if self.filters.contains_key(&filter.id().clone()) {
            self.filters.remove(&filter.id().clone());
        }
    }
}

pub trait BaseTrait {
    fn type_name(&self) -> &str {
        let full_name = type_name::<Self>();
        full_name.split("::").last().unwrap()
    }
    fn type_delete(&self) -> String {
        format!("{}_delete", self.type_name().to_lowercase())
    }
    fn type_add(&self) -> String {
        format!("{}_add", self.type_name().to_lowercase())
    }
    fn type_update(&self) -> String {
        format!("{}_update", self.type_name().to_lowercase())
    }
    fn object_type(&self) -> ObjectType {
        match self.type_name() {
            "Item" => ObjectType::ITEM,
            "Section" => ObjectType::SECTION,
            "Project" => ObjectType::PROJECT,
            "Label" => ObjectType::LABEL,
            _ => ObjectType::FILTER,
        }
    }
    fn object_type_string(&self) -> &str {
        match self.type_name() {
            "Item" => "item",
            "Section" => "section",
            "Project" => "project",
            "Label" => "label",
            _ => "filter",
        }
    }

    fn table_name(&self) -> String {
        format!("{}s", self.type_name())
    }
    fn column_order_name(&self) -> &str {
        match self.type_name() {
            "Item" => "child_order",
            "Section" => "section_order",
            "Project" => "child_order",
            "Label" => "item_order",
            _ => "",
        }
    }
    fn get_update_json(&self, uuid: String, temp_id: String) -> &str {
        ""
    }

    fn get_add_json(&self, temp_id: String, uuid: String) -> &str {
        ""
    }
    fn get_move_json(&self, new_project_id: String, uuid: String) -> &str {
        ""
    }
    fn to_json(&self) -> &str {
        ""
    }

    fn id(&self) -> &str;
    fn set_id(&mut self, id: &str);
    fn id_string(&self) -> &str {
        self.id()
    }

    // signal
    fn deleted(&self) {}
    fn updated(&self, update_id: String) {}
    fn archived(&self) {}
    fn unarchived(&self) {}
    fn filter_added(&mut self, filter: FilterItem) {}
    fn filter_removed(&mut self, filter: FilterItem) {}
    fn filter_updated(&mut self, filter: FilterItem) {}
}
