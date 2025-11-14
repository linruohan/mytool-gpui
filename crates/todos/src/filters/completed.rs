use crate::{BaseObject, Store, enums::FilterType};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    pub base: BaseObject,
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
    pub async fn count(&self, store: Store) -> usize {
        self.count.unwrap_or(store.get_items_completed().await.len())
    }

    pub fn count_updated() {
        // Store::instance().item_added.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        // Store::instance().item_deleted.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        // Store::instance().item_updated.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        // Store::instance().item_archived.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
        // Store::instance().item_unarchived.connect (() => {
        //    _count = Store::instance().get_items_completed ().size;
        //    count_updated ();
        //});
    }
}
