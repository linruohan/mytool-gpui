use gpui::App;
use todos::entity::AttachmentModel;

use crate::core::state::DBState;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    cx.spawn(async move |cx| {
        let db_state = cx.update_global::<DBState, _>(|db_state, _| db_state.clone());
        let store = db_state.get_store_async().await;
        let _ = crate::state_service::add_attachment_with_store(attachment, store).await;
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    cx.spawn(async move |cx| {
        let db_state = cx.update_global::<DBState, _>(|db_state, _| db_state.clone());
        let store = db_state.get_store_async().await;
        let _ = crate::state_service::delete_attachment_with_store(&attachment_id, store).await;
    })
    .detach();
}
