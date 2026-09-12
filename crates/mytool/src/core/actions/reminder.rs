use gpui::App;
use todos::entity::ReminderModel;

use crate::todo_state::DBState;

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |_cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.insert_reminder(reminder).await })
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => tracing::error!("add_reminder failed: {:?}", e),
            Err(join_err) => tracing::error!("add_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |_cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.delete_reminder(&reminder_id).await })
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => tracing::error!("delete_reminder failed: {:?}", e),
            Err(join_err) => tracing::error!("delete_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}
