use gpui::App;
use todos::entity::ReminderModel;

use crate::core::state::get_db_connection;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::add_reminder(reminder, (*db).clone()).await;
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::delete_reminder(&reminder_id, (*db).clone()).await;
    })
    .detach();
}
