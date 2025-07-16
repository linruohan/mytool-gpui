use crate::BaseObject;
use crate::Store;
use crate::Util;
use crate::entity::labels::Model as LabelModel;
use crate::entity::prelude::LabelEntity;
use crate::entity::sources::Model as SourceModel;
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::{BaseTrait, Item};
use sea_orm::prelude::*;
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Label {
    pub model: LabelModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    label_count: Option<usize>,
}

impl Label {
    pub fn name(&self) -> &str {
        &self.model.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.model.name = name.into();
        self
    }
    pub fn color(&self) -> &str {
        &self.model.color
    }
    pub fn set_color(&mut self, color: impl Into<String>) -> &mut Self {
        self.model.color = color.into();
        self
    }
    pub fn item_order(&self) -> i32 {
        self.model.item_order
    }
    pub fn set_item_order(&mut self, order: i32) -> &mut Self {
        self.model.item_order = order;
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
    pub fn backend_type(&self) -> SourceType {
        self.model
            .backend_type
            .as_deref()
            .and_then(|b| serde_json::from_str(b).ok())
            .unwrap_or(SourceType::NONE)
    }
    pub fn set_backend_type(&mut self, backend_type: Option<String>) -> &mut Self {
        self.model.backend_type = backend_type;
        self
    }
    pub fn source_id(&self) -> String {
        self.model
            .source_id
            .as_deref()
            .map(|id| id.to_string())
            .unwrap_or_default()
    }
    pub fn set_source_id(&mut self, source_id: Option<String>) -> &mut Self {
        self.model.source_id = source_id;
        self
    }
}

impl Label {
    pub fn new(db: DatabaseConnection, model: LabelModel) -> Self {
        let base = BaseObject::default();
        Self {
            model,
            base,
            db,
            store: OnceCell::new(),
            label_count: None,
        }
    }

    pub async fn store(&self) -> &Store {
        self.store
            .get_or_init(|| async { Store::new(self.db.clone()).await })
            .await
    }
    pub async fn from_db(db: DatabaseConnection, label_id: &str) -> Result<Self, TodoError> {
        let label = LabelEntity::find_by_id(label_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Label {} not found", label_id)))?;

        Ok(Self::new(db, label))
    }

    pub async fn source_type(&self) -> Option<SourceType> {
        self.source()
            .await
            .and_then(|source| serde_json::from_str(&source.source_type).ok())
            .unwrap_or(Some(SourceType::NONE))
    }
    pub async fn source(&self) -> Option<SourceModel> {
        self.store()
            .await
            .get_source(&self.model.source_id.as_ref()?)
            .await
    }
    async fn label_count(&mut self) -> usize {
        let count = self
            .store()
            .await
            .get_items_by_label(self.id(), false)
            .await
            .len();
        self.label_count = Some(count);
        count
    }
    pub fn set_label_count(&mut self, count: usize) -> &mut Self {
        self.label_count = Some(count);
        self
    }

    pub fn short_name(&self) -> String {
        Util::get_default().get_short_name(&self.model.name, 0)
    }
    pub async fn delete_label(&self) -> Result<u64, TodoError> {
        let items_model = self
            .store()
            .await
            .get_items_by_label(self.id(), false)
            .await;
        for item_model in items_model {
            let mut item = Item::from_db(self.db.clone(), &item_model.id).await?;
            item.delete_item_label(&self.model.id).await;
        }
        self.store().await.delete_label(self.id()).await
    }
}

impl BaseTrait for Label {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
