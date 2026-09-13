use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::SectionModel;

use crate::{core::state::TodoStore, todo_state::DBState};

pub fn add_section(section: Arc<SectionModel>, cx: &mut App) {
    let mut section = section.as_ref().clone();
    if section.id.is_empty() {
        section.id = uuid::Uuid::new_v4().to_string();
    }
    if section.project_id.as_deref().is_some_and(|id| id.is_empty() || id.starts_with("temp_")) {
        section.project_id = None;
    }

    let persist_section = Arc::new(section);
    let section_id = persist_section.id.clone();
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.add_section(persist_section.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let persist_section = persist_section.clone();
                move |store| async move {
                    store.insert_section(persist_section.as_ref().clone()).await
                }
            })
            .await
        {
            Ok(Ok(new_section)) => {
                let arc_section = Arc::new(new_section);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(arc_section);
                });
            },
            Ok(Err(e)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_section(&section_id);
                });
                tracing::error!("add_section failed: {:?}", e);
            },
            Err(join_err) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_section(&section_id);
                });
                tracing::error!("add_section task panicked: {:?}", join_err);
            },
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

/// 批量更新分区（先写内存，再逐条落盘）。
pub fn batch_update_sections(sections: Vec<Arc<SectionModel>>, cx: &mut App) {
    if sections.is_empty() {
        return;
    }
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        for section in &sections {
            todo_store.update_section(section.clone());
        }
    });
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        for section in sections {
            match db_state
                .spawn_store_op({
                    let section = section.clone();
                    move |store| async move { store.update_section(section.as_ref().clone()).await }
                })
                .await
            {
                Ok(Ok(updated)) => {
                    cx.update_global::<TodoStore, _>(|todo_store, _| {
                        todo_store.update_section(Arc::new(updated));
                    });
                },
                Ok(Err(e)) => tracing::error!("batch_update_sections failed: {:?}", e),
                Err(join_err) => {
                    tracing::error!("batch_update_sections task panicked: {:?}", join_err)
                },
            }
        }
    })
    .detach();
}

pub fn set_section_collapsed(section: Arc<SectionModel>, collapsed: bool, cx: &mut App) {
    if section.collapsed == collapsed {
        return;
    }
    let original = section.clone();
    let mut updated = (*section).clone();
    updated.collapsed = collapsed;
    let after = Arc::new(updated);
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.update_section(after.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let after = after.clone();
                move |store| async move { store.update_section(after.as_ref().clone()).await }
            })
            .await
        {
            Ok(Ok(saved)) => {
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(Arc::new(saved));
                });
            },
            Ok(Err(e)) => {
                tracing::error!("set_section_collapsed failed: {:?}", e);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(original.clone());
                });
            },
            Err(join_err) => {
                tracing::error!("set_section_collapsed task panicked: {:?}", join_err);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(original);
                });
            },
        }
    })
    .detach();
}
