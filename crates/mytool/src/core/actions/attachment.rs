use gpui::App;
use todos::entity::AttachmentModel;

use crate::todo_state::DBState;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    let _ = db_state
        .spawn_store_op(move |store| async move { store.insert_attachment(attachment).await });
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    let _ = db_state
        .spawn_store_op(move |store| async move { store.delete_attachment(&attachment_id).await });
}
