use crate::objects::{BaseTrait, Item};
use crate::{BaseObject, Source, Store};

use crate::Project;

use crate::entity::prelude::SectionEntity;
use crate::entity::SectionModel;
use crate::error::TodoError;
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Section {
    pub model: SectionModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
}
impl Section {
    pub fn new(db: DatabaseConnection, model: SectionModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async {
            Store::new(self.db.clone()).await
        }).await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = SectionEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }
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
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
