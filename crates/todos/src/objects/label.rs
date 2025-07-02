use crate::BaseObject;
use crate::Source;
use crate::Store;
use crate::Util;
use crate::entity::labels;
use crate::enums::SourceType;
use crate::objects::BaseTrait;
use sea_orm::prelude::*;
use std::ops::Deref;
#[derive(Clone, Debug)]
pub struct Label {
    pub model: labels::Model,
    base: BaseObject,
    store: Store,
    label_count: Option<usize>,
}

impl Label {
    pub fn new(model: labels::Model, store: Store) -> Self {
        let base = BaseObject::default();
        Self {
            model,
            base,
            store,
            label_count: None,
        }
    }
    pub fn source_type(&self) -> SourceType {
        self.source().map_or(SourceType::NONE, |s| s.source_type())
    }
    pub async fn source(&self) -> Option<Source> {
        self.model
            .source_id
            .as_deref()
            .and(|id| self.store.get_source(id).await)
    }
    fn update_label_count(&mut self, id: &str) -> usize {
        self.label_count = Store::instance().get_items_by_label(id, false).len();
        return self.label_count;
    }
    pub fn set_label_count(&mut self, count: usize) {
        self.label_count = count;
    }

    pub fn short_name(&self) -> String {
        Util::get_default().get_short_name(self.name.clone(), 0)
    }
    pub fn delete_label(&self, id: &str) {
        let items = Store::instance().get_items_by_label(id, false);
        for item in items {
            item.delete_item_label(self.id());
        }
        Store::instance().delete_label(self.clone());
    }
}

impl BaseTrait for Label {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: &str) {
        self.base.id = id.into();
    }
}
