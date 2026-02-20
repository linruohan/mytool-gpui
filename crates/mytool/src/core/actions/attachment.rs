use gpui::App;
use todos::entity::AttachmentModel;

use crate::core::state::get_db_connection;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::add_attachment(attachment, (*db).clone()).await;
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |_cx| {
        let _ = crate::state_service::delete_attachment(&attachment_id, (*db).clone()).await;
    })
    .detach();
}
