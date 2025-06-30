use std::{any::Any, collections::HashMap};

use super::FilterItem;
use crate::{BaseObject, BaseTrait, Store, enums::FilterType};
use derive_builder::Builder;
use uuid::Uuid;
#[derive(Builder, Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    pub base: BaseObject,
    #[builder(default, setter(into, strip_option))]
    pub count: Option<usize>,
}
impl Default for Completed {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Completed".to_string(),
                format!("{};{};{}", "completed", "filters", "logbook"),
                "check-round-outline-symbolic".to_string(),
                FilterType::COMPLETED.to_string(),
            ),
            count: None,
        }
    }
}
impl Completed {
    pub fn count(&self) -> usize {
        self.count
            .unwrap_or(Store::instance().get_items_completed().len())
    }
    pub fn count_updated() {
        //Store::instance().item_added.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        //
        //Store::instance().item_deleted.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        //
        //Store::instance().item_updated.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        //
        //Store::instance().item_archived.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        //
        //Store::instance().item_unarchived.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
    }
}
