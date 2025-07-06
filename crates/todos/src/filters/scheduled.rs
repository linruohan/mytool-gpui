use crate::{enums::FilterType, BaseObject, Store};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduled {
    pub base: BaseObject,
    pub count: usize,
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
            count: 0,
        }
    }
}
impl Scheduled {
    pub async fn count(&self, store: Store) -> usize {
        store.get_items_by_scheduled(false).await.len()
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
