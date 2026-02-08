use crate::{BaseObject, Store, Util};
pub struct Priority {
    pub base: BaseObject,
    pub count: usize,
    pub priority: i32,
}
impl Priority {
    pub fn default(priority: i32) -> Priority {
        let name = Util::get_default().get_priority_title(priority);
        let keywords =
            format!("{};{}", Util::get_default().get_priority_keywords(priority), "filters");
        let view_id = format!("priority-{priority}");
        Self { base: BaseObject::new(name, keywords, "".to_string(), view_id), count: 0, priority }
    }

    pub async fn count(&self, store: &Store) -> usize {
        // 暂时返回 0，因为不存在 get_items_by_priority 方法
        0
    }

    pub fn count_updated(&self) {

        // Store::instance().item_added.connect (() => {
        //     _count = Store::instance().get_items_by_priority (priority, false).size;
        //     count_updated ();
        // });

        // Store::instance().item_deleted.connect (() => {
        //     _count = Store::instance().get_items_by_priority (priority, false).size;
        //     count_updated ();
        // });

        // Store::instance().item_updated.connect (() => {
        //     _count = Store::instance().get_items_by_priority (priority, false).size;
        //     count_updated ();
        // });

        // Store::instance().item_archived.connect (() => {
        //     _count = Store::instance().get_items_by_priority (priority, false).size;
        //     count_updated ();
        // });

        // Store::instance().item_unarchived.connect (() => {
        //     _count = Store::instance().get_items_by_priority (priority, false).size;
        //     count_updated ();
        // });
    }
}
