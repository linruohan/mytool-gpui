use gpui::App;
use todos::entity::AttachmentModel;

use crate::core::state::get_store;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::add_attachment(attachment, (*store.db()).clone()).await;
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |_cx| {
        let _ =
            crate::state_service::delete_attachment(&attachment_id, (*store.db()).clone()).await;
    })
    .detach();
}
