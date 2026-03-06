use gpui::App;
use todos::entity::ReminderModel;

use crate::core::state::get_store;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::add_reminder_with_store(reminder, store).await;
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::delete_reminder_with_store(&reminder_id, store).await;
    })
    .detach();
}
