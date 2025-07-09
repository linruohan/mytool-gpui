use crate::{BaseObject, Store, enums::FilterType};
#[derive(Clone, PartialEq, Eq)]
pub struct Today {
    pub base: BaseObject,
    pub count: usize,
}

impl Default for Today {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Today".to_string(),
                format!("{};{}", "Today", "filters"),
                "star-outline-thick-symbolic".to_string(),
                FilterType::TODAY.to_string(),
            ),
            count: 0,
        }
    }
}

impl Today {
    pub async fn count(&self, store: Store) -> usize {
        store.get_items_by_overdeue_view(false).await.len()
    }

    pub fn today_count_updated(&self) {

        // Services.Store.instance ().item_added.connect (() => {
        //     _today_count = Services.Store.instance ().get_items_by_date (
        //         new GLib.DateTime.now_local (), false).size;
        //     _overdeue_count = Services.Store.instance ().get_items_by_overdeue_view (false).size;
        //     today_count_updated ();
        // });

        // Services.Store.instance ().item_deleted.connect (() => {
        //     _today_count = Services.Store.instance ().get_items_by_date (
        //         new GLib.DateTime.now_local (), false).size;
        //     _overdeue_count = Services.Store.instance ().get_items_by_overdeue_view (false).size;
        //     today_count_updated ();
        // });

        // Services.Store.instance ().item_archived.connect (() => {
        //     _today_count = Services.Store.instance ().get_items_by_date (
        //         new GLib.DateTime.now_local (), false).size;
        //     _overdeue_count = Services.Store.instance ().get_items_by_overdeue_view (false).size;
        //     today_count_updated ();
        // });

        // Services.Store.instance ().item_unarchived.connect (() => {
        //     _today_count = Services.Store.instance ().get_items_by_date (
        //         new GLib.DateTime.now_local (), false).size;
        //     _overdeue_count = Services.Store.instance ().get_items_by_overdeue_view (false).size;
        //     today_count_updated ();
        // });

        // Services.Store.instance ().item_updated.connect (() => {
        //     _today_count = Services.Store.instance ().get_items_by_date (
        //         new GLib.DateTime.now_local (), false).size;
        //     _overdeue_count = Services.Store.instance ().get_items_by_overdeue_view (false).size;
        //     today_count_updated ();
        // });
    }
}
