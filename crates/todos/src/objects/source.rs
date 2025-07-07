use crate::entity::prelude::SourceEntity;
use crate::entity::SourceModel;
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::BaseTrait;
use crate::services::Store;
use crate::BaseObject;
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Source {
    pub model: SourceModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
}

impl Source {
    pub fn new(db: DatabaseConnection, model: SourceModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async {
            Store::new(self.db.clone()).await
        }).await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = SourceEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }
    pub fn source_type(&self) -> SourceType {
        SourceType::parse(Some(&self.model.source_type))
    }
    pub fn header_text(&self) -> String {
        self.model.display_name.clone().unwrap_or_default()
    }
}

impl BaseTrait for Source {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
