use crate::{BaseObject, Store, enums::FilterType};
use derive_builder::Builder;
#[derive(Builder, Debug, Clone, PartialEq, Eq)]
pub struct Scheduled {
    pub base: BaseObject,
    #[builder(default, setter(into, strip_option))]
    pub count: Option<usize>,
}
impl Default for Scheduled {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Scheduled".to_string(),
                format!("{};{};{}", "scheduled", "upcoming", "filters"),
                "month-symbolic".to_string(),
                FilterType::SCHEDULED.to_string(),
            ),
            count: None,
        }
    }
}
impl Scheduled {
    pub fn count(&self) -> usize {
        self.count
            .unwrap_or(Store::instance().get_items_by_scheduled(false).len())
    }

    pub fn scheduled_count_updated(&self) {

        // Services.Store.instance ().item_added.connect (() => {
        //     _scheduled_count = Services.Store.instance ().get_items_by_scheduled (false).size;
        //     scheduled_count_updated ();
        // });

        // Services.Store.instance ().item_deleted.connect (() => {
        //     _scheduled_count = Services.Store.instance ().get_items_by_scheduled (false).size;
        //     scheduled_count_updated ();
        // });

        // Services.Store.instance ().item_updated.connect (() => {
        //     _scheduled_count = Services.Store.instance ().get_items_by_scheduled (false).size;
        //     scheduled_count_updated ();
        // });

        // Services.Store.instance ().item_archived.connect (() => {
        //     _scheduled_count = Services.Store.instance ().get_items_by_scheduled (false).size;
        //     scheduled_count_updated ();
        // });

        // Services.Store.instance ().item_unarchived.connect (() => {
        //     _scheduled_count = Services.Store.instance ().get_items_by_scheduled (false).size;
        //     scheduled_count_updated ();
        // });
    }
}
