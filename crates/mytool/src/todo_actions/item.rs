use std::sync::Arc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;
use tracing::error;

use crate::todo_state::{DBState, ProjectState, TodoStore};

// 刷新指定项目的 items（仅在有活跃项目时需要）
async fn refresh_project_items(project_id: &str, cx: &mut AsyncApp, db: DatabaseConnection) {
    let items = crate::service::get_items_by_project_id(project_id, db).await;
    let arc_items: Vec<_> = items.iter().map(|item| Arc::new(item.clone())).collect();

    cx.update_global::<ProjectState, _>(|state, _| {
        if let Some(active) = &state.active_project
            && active.id == project_id
        {
            state.items = arc_items;
        }
    });
}

// 添加 item（使用增量更新，性能最优）
pub fn add_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::add_item(item.clone(), db.clone()).await {
            Ok(new_item) => {
                // 增量更新：只添加新任务到 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(Arc::new(new_item));
                });
            },
            Err(e) => {
                error!("add_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

// 修改 item（使用增量更新，性能最优）
pub fn update_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let active_project = cx.global::<ProjectState>().active_project.clone();

    cx.spawn(async move |cx| {
        match crate::service::mod_item(item.clone(), db.clone()).await {
            Ok(updated_item) => {
                // 增量更新：只更新修改的任务
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
                // 如果有活跃项目，刷新项目列表
                if let Some(active) = active_project {
                    refresh_project_items(&active.id, cx, db).await;
                }
            },
            Err(e) => {
                error!("update_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

// 删除 item（使用增量更新，性能最优）
pub fn delete_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    let item_id = item.id.clone();
    cx.spawn(async move |cx| {
        match crate::service::del_item(item.clone(), db.clone()).await {
            Ok(_) => {
                // 增量更新：只删除指定的任务
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_item(&item_id);
                });
            },
            Err(e) => {
                error!("delete_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

// 完成任务（使用增量更新，性能最优）
pub fn completed_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::finish_item(item.clone(), true, false, db.clone()).await {
            Ok(_) => {
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.checked = true;
                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                error!("completed_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

// 取消完成任务（使用增量更新，性能最优）
pub fn uncompleted_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::finish_item(item.clone(), false, false, db.clone()).await {
            Ok(_) => {
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.checked = false;
                updated_item.completed_at = None;
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                error!("uncompleted_item failed: {:?}", e);
            },
        }
    })
    .detach();
}

// 置顶/取消置顶任务（使用增量更新，性能最优）
pub fn set_item_pinned(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::service::pin_item(item.clone(), pinned, db.clone()).await {
            Ok(_) => {
                // 增量更新：更新本地状态
                let mut updated_item = (*item).clone();
                updated_item.pinned = pinned;
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(updated_item));
                });
            },
            Err(e) => {
                error!("set_item_pinned failed: {:?}", e);
            },
        }
    })
    .detach();
}
