use crate::BaseObject;
use crate::Store;
use crate::Util;
use crate::entity::{labels, sources};
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::generate_accessors;
use crate::objects::BaseTrait;
use sea_orm::prelude::*;

#[derive(Clone, Debug)]
pub struct Label {
    pub model: labels::Model,
    base: BaseObject,
    store: Store,
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
        self.model.color.as_deref().unwrap_or_default()
    }
    pub fn set_color(&mut self, color: impl Into<String>) -> &mut Self {
        self.model.color = Some(color.into());
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
    pub fn backend_type(&self) -> Option<SourceType> {
        self.model
            .backend_type
            .as_deref()
            .and(|b| SourceType::from_str(b).unwrap_or(SourceType::NONE))
    }
    pub fn set_backend_type(&mut self, backend_type: Option<String>) -> &mut Self {
        self.model.backend_type = backend_type;
        self
    }
    pub fn source_id(&self) -> String {
        self.model
            .source_id
            .clone()
            .unwrap_or(|| SourceType::LOCAL.to_string())
    }
    pub fn set_source_id(&mut self, source_id: Option<String>) -> &mut Self {
        self.model.source_id = source_id;
        self
    }
}

impl Label {
    pub fn new(db: DatabaseConnection, model: labels::Model) -> Self {
        let base = BaseObject::default();
        let store = Store::new(db);
        Self {
            model,
            base,
            store,
            label_count: None,
        }
    }

    pub async fn source_type(&self) -> SourceType {
        self.source()
            .await
            .map_or(SourceType::NONE, |s| s.source_type())
    }
    pub async fn source(&self) -> Result<sources::Model, TodoError> {
        let id = self.source_id().ok_or(TodoError::IDNotFound)?;
        self.store.get_source(&id).await?
    }
    fn label_count(&mut self) -> usize {
        self.label_count = self.store.get_items_by_label(self.id, false).len();
        return self.label_count;
    }
    pub fn set_label_count(&mut self, count: usize) -> &mut Self {
        self.label_count = count;
        self
    }

    pub fn short_name(&self) -> String {
        Util::get_default().get_short_name(self.name.clone(), 0)
    }
    pub fn delete_label(&self) {
        let items = self.store.get_items_by_label(self.id(), false);
        for item in items {
            item.delete_item_label(self.id());
        }
        self.store.delete_label(self.clone());
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
