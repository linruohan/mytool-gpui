use gpui::App;
use todos::entity::AttachmentModel;

use crate::todo_state::DBState;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |_cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.insert_attachment(attachment).await })
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => tracing::error!("add_attachment failed: {:?}", e),
            Err(join_err) => tracing::error!("add_attachment task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |_cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.delete_attachment(&attachment_id).await })
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => tracing::error!("delete_attachment failed: {:?}", e),
            Err(join_err) => tracing::error!("delete_attachment task panicked: {:?}", join_err),
        }
    })
    .detach();
}
