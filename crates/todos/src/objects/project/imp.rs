use crate::enums::SourceType;
use crate::objects::{BaseTrait, Item};
use crate::schema::projects;
use crate::{Source, Store};
use diesel::Queryable;
use diesel::prelude::*;
use diesel::row::NamedRow;
use serde::{Deserialize, Serialize};

#[derive(
    QueryableByName, Queryable, PartialEq, Insertable, Clone, Eq, Selectable, Serialize, Debug,
)]
#[diesel(table_name = projects)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Project {
    pub id: Option<String>,
    pub parent_id: Option<String>,
    pub name: String,
    pub source_id: Option<String>,
    pub color: Option<String>,
    pub backend_type: Option<String>,
    pub inbox_project: Option<i32>,
    pub team_inbox: Option<i32>,
    pub child_order: Option<i32>,
    pub is_deleted: Option<i32>,
    pub is_archived: Option<i32>,
    pub is_favorite: Option<i32>,
    pub shared: Option<i32>,
    pub view_style: Option<String>,
    pub sort_order: Option<i32>,
    pub collapsed: Option<i32>,
    pub icon_style: Option<String>,
    pub emoji: Option<String>,
    pub show_completed: Option<i32>,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub inbox_section_hidded: Option<i32>,
    pub sync_id: Option<String>,
}

impl Project {
    pub(crate) fn item_added(&self, item: &Item) {
        todo!()
    }
    pub(crate) fn item_deleted(&self, item: &Item) {
        todo!()
    }
}

impl Project {
    pub fn default() -> Project {
        Self {
            id: None,
            parent_id: None,
            name: String::new(),
            source_id: None,
            color: None,
            backend_type: None,
            inbox_project: Some(0),
            team_inbox: Some(0),
            child_order: Some(0),
            is_deleted: Some(0),
            is_archived: Some(0),
            is_favorite: Some(0),
            shared: Some(0),
            view_style: None,
            sort_order: Some(0),
            collapsed: Some(0),
            icon_style: None,
            emoji: None,
            show_completed: Some(1),
            description: None,
            due_date: None,
            inbox_section_hidded: Some(0),
            sync_id: None,
        }
    }
    pub fn is_deleted(&self) -> bool {
        self.is_deleted.unwrap_or(0) > 0
    }
    pub fn is_favorite(&self) -> bool {
        self.is_favorite.unwrap_or(0) > 0
    }
    pub fn project_count(&self) -> usize {
        let items = Store::instance().get_items_by_project(self);
        items
            .iter()
            .filter(|i| !i.checked() || !i.was_archived())
            .count()
    }
    pub(crate) fn is_inbox_project(&self) -> bool {
        todo!()
    }
    pub(crate) fn is_archived(&self) -> bool {
        self.is_archived.unwrap_or(0) > 0
    }
    pub fn source_type(&self) -> SourceType {
        self.source().map_or(SourceType::NONE, |s| s.source_type())
    }
    pub(crate) fn update_count(&self) {
        todo!()
    }
    pub fn parent(&self) -> Option<Project> {
        self.parent_id
            .as_deref()
            .and_then(|id| Store::instance().get_project(id))
    }
    pub fn add_subproject(&self, subproject: &Project) {
        Store::instance().insert_project(&subproject);
    }
    pub fn source(&self) -> Option<Source> {
        self.source_id
            .as_deref()
            .and_then(|id| Store::instance().get_source(id))
    }
}
impl BaseTrait for Project {
    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }
    fn set_id(&mut self, id: &str) {
        self.id = Some(id.into());
    }
}
