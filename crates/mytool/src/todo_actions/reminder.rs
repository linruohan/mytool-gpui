use gpui::App;
use todos::entity::ReminderModel;

use crate::todo_state::DBState;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |_cx| {
        let db = conn.lock().await;
        let _ = crate::service::add_reminder(reminder, (*db).clone()).await;
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |_cx| {
        let db = conn.lock().await;
        let _ = crate::service::delete_reminder(&reminder_id, (*db).clone()).await;
    })
    .detach();
}
