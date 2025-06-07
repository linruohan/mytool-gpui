use crate::{BaseObject, Store, enums::FilterType};
use derive_builder::Builder;
#[derive(Builder, Clone, PartialEq, Eq)]
pub struct Today {
    pub base: BaseObject,
    #[builder(default, setter(into, strip_option))]
    pub count: Option<usize>,
}

impl Today {
    pub fn default() -> Today {
        Self {
            base: BaseObject::new(
                "Today".to_string(),
                format!("{};{}", "Today", "filters"),
                "star-outline-thick-symbolic".to_string(),
                FilterType::TODAY.to_string(),
            ),
            count: None,
        }
    }
    pub fn count(&self) -> usize {
        self.count
            .clone()
            .unwrap_or(Store::instance().get_items_by_overdeue_view(false).len())
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
