use crate::BaseObject;
use crate::Source;
use crate::Store;
use crate::Util;
use crate::entity::labels;
use crate::entity::labels::ActiveModel;
use crate::entity::prelude::*;
use crate::enums::SourceType;
use crate::objects::BaseTrait;
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, Condition, IntoActiveModel, QueryOrder, QueryTrait};
use std::ops::Deref;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub base: BaseObject,
    pub color: String,
    pub item_order: i32,
    pub is_deleted: bool,
    pub is_favorite: bool,
    pub backend_type: SourceType,
    pub source_id: String,
    label_count: usize,
    name: String,
}
impl Deref for Label {
    type Target = BaseObject;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl Label {
    pub fn source_type(&self) -> SourceType {
        self.source().map_or(SourceType::NONE, |s| s.source_type())
    }
    pub fn source(&self) -> Option<Source> {
        Store::instance().get_source(&self.source_id)
    }
    fn update_label_count(&mut self) -> usize {
        self.label_count = Store::instance().get_items_by_label(self.id(), false).len();
        return self.label_count;
    }
    pub fn set_label_count(&mut self, count: usize) {
        self.label_count = count;
    }

    pub fn short_name(&self) -> String {
        Util::get_default().get_short_name(self.name.clone(), 0)
    }
    pub fn delete_label(&self) {
        let items = Store::instance().get_items_by_label(self.id(), false);
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
