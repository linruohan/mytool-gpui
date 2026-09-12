use std::sync::Arc;

use gpui::{App, AsyncApp, BorrowAppContext};
use todos::entity::{ItemModel, ProjectModel};

use crate::{core::state::TodoStore, todo_state::DBState};

/// 切换项目：只更新内存中的活跃项目。列表数据已在冷加载进 `TodoStore`。
pub fn load_project_items(project: Arc<ProjectModel>, cx: &mut App) {
    tracing::debug!(
        "load_project_items，project_id: {}, project_name: {}",
        project.id,
        project.name
    );

    if project.id.is_empty() {
        tracing::error!("load_project_items: project_id 为空，跳过");
        return;
    }

    cx.update_global::<TodoStore, _>(|state, _| {
        state.set_active_project(Some(project));
    });
}

fn apply_saved_item(cx: &mut AsyncApp, previous_id: &str, saved: ItemModel) {
    let saved = Arc::new(saved);
    cx.update_global::<TodoStore, _>(|state, _| {
        if saved.id != previous_id {
            state.replace_item_id(previous_id, saved);
        } else {
            state.update_item(saved);
        }
    });
}

/// 添加 item 到项目（异步落盘，内存由调用方乐观更新或由此处回写）
pub fn add_project_item(_project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let item_clone = item.clone();
    let previous_id = item.id.clone();
    let db_state = cx.global::<DBState>().clone();

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.insert_item(item_clone.as_ref().clone(), true).await
            })
            .await
        {
            Ok(Ok(saved)) => {
                tracing::debug!("add_project_item: item added successfully");
                apply_saved_item(cx, &previous_id, saved);
            },
            Ok(Err(e)) => tracing::error!("add_project_item failed: {:?}", e),
            Err(join_err) => tracing::error!("add_project_item task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn update_project_item(_project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let previous_id = item.id.clone();
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.update_item(item.as_ref().clone(), "").await
            })
            .await
        {
            Ok(Ok(saved)) => apply_saved_item(cx, &previous_id, saved),
            Ok(Err(e)) => tracing::error!("update_project_item failed: {:?}", e),
            Err(join_err) => tracing::error!("update_project_item task panicked: {:?}", join_err),
        }
    })
    .detach();
}

/// 删除项目中的 item（异步落盘，成功后从内存移除）
pub fn delete_project_item(_project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let item_id = item.id.clone();
    let db_state = cx.global::<DBState>().clone();

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op({
                let item_id = item_id.clone();
                move |store| async move { store.delete_item(&item_id).await }
            })
            .await
        {
            Ok(Ok(_)) => {
                tracing::debug!("delete_project_item: item deleted successfully");
                cx.update_global::<TodoStore, _>(|state, _| {
                    state.remove_item(&item_id);
                });
            },
            Ok(Err(e)) => tracing::error!("delete_project_item failed: {:?}", e),
            Err(join_err) => tracing::error!("delete_project_item task panicked: {:?}", join_err),
        }
    })
    .detach();
}
