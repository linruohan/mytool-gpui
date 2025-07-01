use std::{any::Any, collections::HashMap};

use uuid::Uuid;

use super::FilterItem;
use crate::{BaseObject, BaseTrait};
use crate::{Store, enums::FilterType};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    pub base: BaseObject,
    pub count: usize,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Labels".to_string(),
                format!("{};{}", "label", "filters"),
                "tag-outline-symbolic".to_string(),
                FilterType::LABELS.to_string(),
            ),
            count: 0,
        }
    }
}

impl Labels {
    pub fn count(&self) -> usize {
        Store::instance().get_items_has_labels().len()
    }
    pub fn count_updated(&self) {

        // Store::instance().label_added.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().label_deleted.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().label_updated.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().item_added.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().item_deleted.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().item_archived.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().item_unarchived.connect ((item) => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });

        // Store::instance().item_updated.connect (() => {
        //     _count = Store::instance().get_items_has_labels ().size;
        //     count_updated ();
        // });
    }
}
