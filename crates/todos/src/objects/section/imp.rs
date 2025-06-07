use crate::objects::{BaseTrait, Item};
use crate::schema::sections;
use crate::{Source, Store};
use derive_builder::Builder;
use diesel::Queryable;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Project;
#[derive(
    QueryableByName,
    Builder,
    Clone,
    Queryable,
    Insertable,
    PartialEq,
    Eq,
    Selectable,
    Serialize,
    Debug,
)]
#[diesel(table_name = sections)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Section {
    pub id: Option<String>,
    pub name: Option<String>,
    pub archived_at: Option<String>,
    pub added_at: Option<String>,
    pub project_id: Option<String>,
    pub section_order: Option<i32>,
    pub collapsed: Option<i32>,
    pub is_deleted: Option<i32>,
    pub is_archived: Option<i32>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub hidded: Option<i32>,
}

impl Section {
    pub fn item_added(&self, item: &Item) {
        todo!()
    }
    pub fn item_deleted(&self, item: &Item) {
        todo!()
    }
    pub fn section_count_updated(&self) {
        todo!()
    }
}
impl Section {
    pub fn project(&self) -> Option<Project> {
        Store::instance().get_project(self.project_id.as_ref()?) // Assuming Store has a method to get project by ID
    }
    pub fn items(&self) -> Vec<Item> {
        let mut items = Store::instance().get_item_by_baseobject(Box::new(self.clone()));
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }
    pub fn is_archived(&self) -> bool {
        self.is_archived.unwrap_or(0) > 0
    }
    pub(crate) fn update_count(&self) {
        todo!()
    }
    pub fn was_archived(&self) -> bool {
        self.project()
            .as_ref()
            .map_or(self.is_archived(), |p| p.is_archived())
    }
    pub fn source(&self) -> Option<Source> {
        self.project()
            .as_ref()
            .map_or(Some(Source::default()), |p| p.source())
    }
}

impl BaseTrait for Section {
    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }
    fn set_id(&mut self, id: &str) {
        self.id = Some(id.into());
    }
}
