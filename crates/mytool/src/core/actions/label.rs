use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::LabelModel;
use tracing::{debug, error};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::TodoStore,
    },
    todo_state::DBState,
};

pub fn add_label(label: Arc<LabelModel>, cx: &mut App) {
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "add_label");
        error!("{}", context.format_user_message());
        return;
    }

    let db_state = cx.global::<DBState>().clone();
    let label_id = label.id.clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(
                move |store| async move { store.insert_label(label.as_ref().clone()).await },
            )
            .await
        {
            Ok(Ok(new_label)) => {
                debug!("Successfully added label: {}", new_label.id);
                let arc_label = Arc::new(new_label);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_label(arc_label);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_label",
                    &label_id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                error!("add_label task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

pub fn update_label(label: Arc<LabelModel>, cx: &mut App) {
    if let Err(e) = validation::validate_label_name(&label.name) {
        let context = ErrorHandler::handle_with_location(e, "update_label");
        error!("{}", context.format_user_message());
        return;
    }

    let db_state = cx.global::<DBState>().clone();
    let label_id = label.id.clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(
                move |store| async move { store.update_label(label.as_ref().clone()).await },
            )
            .await
        {
            Ok(Ok(new_label)) => {
                debug!("Successfully updated label: {} (name: {})", new_label.id, new_label.name);
                let arc_label = Arc::new(new_label);
                cx.update_global::<TodoStore, _>(|todo_store, _cx| {
                    todo_store.update_label(arc_label.clone());
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "update_label",
                    &label_id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                error!("update_label task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

pub fn delete_label(label: Arc<LabelModel>, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let label_id = label.id.clone();
                move |store| async move { store.delete_label(&label_id).await }
            })
            .await
        {
            Ok(Ok(_)) => {
                debug!("Successfully deleted label: {}", label.id);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_label(&label.id);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "delete_label",
                    &label.id,
                );
                error!("{}", context.format_user_message());
            },
            Err(join_err) => {
                error!("delete_label task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

pub fn set_label_favorite(label: Arc<LabelModel>, is_favorite: bool, cx: &mut App) {
    if label.is_favorite == is_favorite {
        return;
    }
    let original = label.clone();
    let mut updated = (*label).clone();
    updated.is_favorite = is_favorite;
    let after = Arc::new(updated);
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.update_label(after.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let after = after.clone();
                move |store| async move { store.update_label(after.as_ref().clone()).await }
            })
            .await
        {
            Ok(Ok(saved)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_label(Arc::new(saved));
                });
            },
            Ok(Err(e)) => {
                error!("set_label_favorite failed: {:?}", e);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_label(original.clone());
                });
            },
            Err(join_err) => {
                error!("set_label_favorite task panicked: {:?}", join_err);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_label(original);
                });
            },
        }
    })
    .detach();
}
