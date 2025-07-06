use crate::{enums::FilterType, BaseObject, Store};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pinboard {
    pub base: BaseObject,
}
impl Default for Pinboard {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Pinboard".to_string(),
                format!("{};{}", "Pinboard", "filters"),
                "pin-symbolic".to_string(),
                FilterType::PINBOARD.to_string(),
            ),
        }
    }
}

impl Pinboard {
    pub async fn pinboard_count(&self, store: Store) -> usize {
        store.get_items_pinned(false).await.len()
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
