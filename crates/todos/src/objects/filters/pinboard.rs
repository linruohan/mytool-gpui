use std::fmt::format;

use crate::{BaseObject, Store, enums::FilterType};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pinboard {
    pub base: BaseObject,
}
impl Pinboard {
    pub fn default() -> Pinboard {
        Self {
            base: BaseObject::new(
                "Pinboard".to_string(),
                format!("{};{}", "Pinboard", "filters"),
                "pin-symbolic".to_string(),
                FilterType::PINBOARD.to_string(),
            ),
        }
    }
    pub fn pinboard_count(&self) -> usize {
        Store::instance().get_items_pinned(false).len()
    }

    pub fn pinboard_count_updated(&self) {
        // Store::instance().item_added.connect (() => {
        //     _pinboard_count = Store::instance().get_items_pinned (false).size;
        //     pinboard_count_updated ();
        // });

        // Store::instance().item_deleted.connect (() => {
        //     _pinboard_count = Store::instance().get_items_pinned (false).size;
        //     pinboard_count_updated ();
        // });

        // Store::instance().item_updated.connect (() => {
        //     _pinboard_count = Store::instance().get_items_pinned (false).size;
        //     pinboard_count_updated ();
        // });

        // Store::instance().item_archived.connect (() => {
        //     _pinboard_count = Store::instance().get_items_pinned (false).size;
        //     pinboard_count_updated ();
        // });

        // Store::instance().item_unarchived.connect (() => {
        //     _pinboard_count = Store::instance().get_items_pinned (false).size;
        //     pinboard_count_updated ();
        // });
    }
}
