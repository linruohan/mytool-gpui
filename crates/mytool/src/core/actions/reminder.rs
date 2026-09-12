use gpui::App;
use todos::entity::ReminderModel;

use crate::todo_state::DBState;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    let _ =
        db_state.spawn_store_op(move |store| async move { store.insert_reminder(reminder).await });
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    let _ = db_state
        .spawn_store_op(move |store| async move { store.delete_reminder(&reminder_id).await });
}
