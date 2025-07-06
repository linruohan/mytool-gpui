use crate::entity::prelude::ProjectEntity;
use crate::entity::ProjectModel;
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::BaseTrait;
use crate::{BaseObject, Source, Store};
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Project {
    pub model: ProjectModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    project_count: Option<usize>,
}

impl Project {
    pub fn id(&self) -> &str {
        &self.model.id
    }
    pub fn set_id(&mut self, id: String) -> &mut Self {
        self.model.id = id;
        self
    }
    pub fn name(&self) -> &str {
        &self.model.name
    }
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.model.name = name;
        self
    }
    pub fn is_deleted(&self) -> bool {
        self.model.is_deleted
    }
    pub fn set_is_deleted(&mut self, is_deleted: bool) -> &mut Self {
        self.model.is_deleted = is_deleted;
        self
    }
    pub fn is_favorite(&self) -> bool {
        self.model.is_favorite
    }
    pub fn set_is_favorite(&mut self, is_favorite: bool) -> &mut Self {
        self.model.is_favorite = is_favorite;
        self
    }
    pub fn is_archived(&self) -> bool {
        self.model.is_archived
    }
    pub fn set_is_archived(&mut self, is_archived: bool) -> &mut Self {
        self.model.is_archived = is_archived;
        self
    }
    pub fn parent_id(&self) -> Option<&str> {
        self.model.parent_id.as_deref()
    }
    pub fn set_parent_id(&mut self, parent_id: Option<String>) -> &mut Self {
        self.model.parent_id = parent_id;
        self
    }
    pub fn source_id(&self) -> Option<&str> {
        self.model.source_id.as_deref()
    }
    pub fn set_source_id(&mut self, source_id: Option<String>) -> &mut Self {
        self.model.source_id = source_id;
        self
    }
    pub fn display_name(&self) -> String {
        self.model.display_name.clone().unwrap_or_else(|| self.name().to_string())
    }
    pub fn set_display_name(&mut self, display_name: Option<String>) -> &mut Self {
        self.model.display_name = display_name;
        self
    }
}

impl Project {
    pub fn new(db: DatabaseConnection, model: ProjectModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new(), project_count: None }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async {
            Store::new(self.db.clone()).await
        }).await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = ProjectEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
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
        Store::instance().insert_project(subproject);
    }
    pub fn source(&self) -> Option<Source> {
        self.source_id
            .as_deref()
            .and_then(|id| Store::instance().get_source(id))
    }
}

impl BaseTrait for Project {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
