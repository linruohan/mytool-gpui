use gpui::App;
use todos::entity::ReminderModel;

use crate::core::state::get_store;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ = store.insert_reminder(reminder).await;
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ = store.delete_reminder(&reminder_id).await;
    })
    .detach();
}
