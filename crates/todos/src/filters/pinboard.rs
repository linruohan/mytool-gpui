use crate::{BaseObject, Store, enums::FilterType};
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
    pub async fn count(&self, store: &Store) -> usize {
        // 暂时返回 0，因为不存在 get_items_pinned 方法
        0
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
