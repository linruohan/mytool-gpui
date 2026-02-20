use std::sync::Arc;

use gpui::App;
use todos::entity::LabelModel;
use tracing::{error, info};

use crate::{
    error_handler::{AppError, ErrorHandler, validation},
    todo_state::{DBState, TodoStore},
};

// 添加label
pub fn add_label(label: Arc<LabelModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "add_label");
        error!("{}", context.format_user_message());
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_label(label.clone(), db.clone()).await {
            Ok(new_label) => {
                info!("Successfully added label: {}", new_label.id);
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_label(arc_label);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_label",
                    &label.id,
                );
                error!("{}", context.format_user_message());
            }
        }
    })
    .detach();
}

// 修改label
pub fn update_label(label: Arc<LabelModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "update_label");
        error!("{}", context.format_user_message());
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::mod_label(label.clone(), db.clone()).await {
            Ok(new_label) => {
                info!("Successfully updated label: {}", new_label.id);
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_label(arc_label);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "update_label",
                    &label.id,
                );
                error!("{}", context.format_user_message());
            }
        }
    })
    .detach();
}

// 删除label
pub fn delete_label(label: Arc<LabelModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let label_id = label.id.clone();
    
    cx.spawn(async move |cx| {
        match crate::state_service::del_label(label.clone(), db.clone()).await {
            Ok(_) => {
                info!("Successfully deleted label: {}", label_id);
                // 增量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_label(&label_id);
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_label",
                    &label_id,
                );
                error!("{}", context.format_user_message());
            }
        }
    })
    .detach();
}
