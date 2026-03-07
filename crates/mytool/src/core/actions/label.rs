use std::sync::Arc;

use gpui::App;
use todos::entity::LabelModel;
use tracing::{error, info};

use crate::core::{
    error_handler::{AppError, ErrorHandler, validation},
    state::{TodoStore, get_store},
};

// 添加 label
pub fn add_label(label: Arc<LabelModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "add_label");
        error!("{}", context.format_user_message());
        return;
    }

    let store = get_store(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::add_label_with_store(label.clone(), store).await {
            Ok(new_label) => {
                info!("Successfully added label: {}", new_label.id);
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_label(arc_label);
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_label",
                    &label.id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 修改 label
pub fn update_label(label: Arc<LabelModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "update_label");
        error!("{}", context.format_user_message());
        return;
    }

    let store = get_store(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::mod_label_with_store(label.clone(), store).await {
            Ok(new_label) => {
                info!("Successfully updated label: {} (name: {})", new_label.id, new_label.name);
                // 增量更新 TodoStore
                let arc_label = Arc::new(new_label);
                cx.update_global::<TodoStore, _>(|todo_store, _cx| {
                    todo_store.update_label(arc_label.clone());
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "update_label",
                    &label.id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}

// 删除 label
pub fn delete_label(label: Arc<LabelModel>, cx: &mut App) {
    let store = get_store(cx);
    let label_id = label.id.clone();

    cx.spawn(async move |cx| {
        match crate::state_service::del_label_with_store(label.clone(), store).await {
            Ok(_) => {
                info!("Successfully deleted label: {}", label_id);
                // 增量更新 TodoStore
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_label(&label_id);
                });
            },
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "delete_label",
                    &label_id,
                );
                error!("{}", context.format_user_message());
            },
        }
    })
    .detach();
}
