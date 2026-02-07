use gpui::App;
use todos::entity::AttachmentModel;

use crate::todo_state::DBState;

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |_cx| {
        let _ = crate::service::add_attachment(attachment, db.clone()).await;
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |_cx| {
        let _ = crate::service::delete_attachment(&attachment_id, db.clone()).await;
    })
    .detach();
}
