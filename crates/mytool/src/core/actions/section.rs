use std::sync::Arc;

use gpui::App;
use todos::entity::SectionModel;

use crate::{core::state::TodoStore, todo_state::DBState};

pub fn add_section(section: Arc<SectionModel>, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.insert_section(section.as_ref().clone()).await
            })
            .await
        {
            Ok(Ok(new_section)) => {
                let arc_section = Arc::new(new_section);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_section(arc_section);
                });
            },
            Ok(Err(e)) => tracing::error!("add_section failed: {:?}", e),
            Err(join_err) => tracing::error!("add_section task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn update_section(section: Arc<SectionModel>, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.update_section(section.as_ref().clone()).await
            })
            .await
        {
            Ok(Ok(updated_section)) => {
                let arc_section = Arc::new(updated_section);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(arc_section);
                });
            },
            Ok(Err(e)) => tracing::error!("update_section failed: {:?}", e),
            Err(join_err) => tracing::error!("update_section task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn delete_section(section: Arc<SectionModel>, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let section_id = section.id.clone();
                move |store| async move { store.delete_section(&section_id).await }
            })
            .await
        {
            Ok(Ok(_)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_section(&section.id);
                });
            },
            Ok(Err(e)) => tracing::error!("delete_section failed: {:?}", e),
            Err(join_err) => tracing::error!("delete_section task panicked: {:?}", join_err),
        }
    })
    .detach();
}
