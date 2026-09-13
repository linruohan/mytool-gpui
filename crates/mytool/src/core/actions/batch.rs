//! 批量操作模块
//!
//! 提供批量更新任务的功能，减少数据库 I/O 操作。

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use todos::entity::ItemModel;
use tracing::{debug, error};

use super::optimistic::{
    complete_item_optimistic, delete_item_optimistic, set_item_pinned_optimistic,
};
use crate::{
    core::state::{ItemSelection, TodoStore, UndoEntry, UndoStack},
    todo_state::DBState,
};

/// 批量更新任务
pub fn batch_update_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    if items.is_empty() {
        return;
    }

    let parts: Vec<UndoEntry> = {
        let store = cx.global::<TodoStore>();
        items
            .iter()
            .filter_map(|after| {
                store
                    .get_item(&after.id)
                    .map(|before| UndoEntry::Updated { before, after: after.clone() })
            })
            .collect()
    };
    if !parts.is_empty() {
        cx.update_global::<UndoStack, _>(|stack, _| {
            stack.record(UndoEntry::Batch(parts));
        });
    }

    let item_count = items.len();
    debug!("Batch updating {} items", item_count);

    cx.update_global::<TodoStore, _>(|todo_store, _| {
        for item in &items {
            todo_store.update_item(item.clone());
        }
    });

    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                let items_vec: Vec<ItemModel> = items.iter().map(|item| (**item).clone()).collect();
                store.batch_update_items(items_vec).await
            })
            .await
        {
            Ok(Ok(updated_items)) => {
                debug!("Successfully updated {} items in batch", updated_items.len());
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    for item in updated_items {
                        todo_store.update_item(Arc::new(item));
                    }
                });
            },
            Ok(Err(e)) => {
                error!("Batch update items failed: {:?}", e);
            },
            Err(join_err) => {
                error!("Batch update task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

fn selected_items(cx: &App) -> Vec<Arc<ItemModel>> {
    let ids = cx.global::<ItemSelection>().ids().clone();
    let store = cx.global::<TodoStore>();
    ids.iter().filter_map(|id| store.get_item(id)).collect()
}

/// 批量完成当前选中任务
pub fn batch_complete_selected(cx: &mut App) -> usize {
    let items = selected_items(cx);
    let count = items.len();
    for item in items {
        if !item.checked {
            complete_item_optimistic(item, true, cx);
        }
    }
    cx.update_global::<ItemSelection, _>(|sel, _| sel.clear());
    count
}

/// 批量删除当前选中任务
pub fn batch_delete_selected(cx: &mut App) -> usize {
    let items = selected_items(cx);
    let count = items.len();
    for item in items {
        delete_item_optimistic(item, cx);
    }
    cx.update_global::<ItemSelection, _>(|sel, _| sel.clear());
    count
}

/// 批量置顶 / 取消置顶（未置顶不少于一半时一律置顶）
pub fn batch_pin_selected(cx: &mut App) -> usize {
    let items = selected_items(cx);
    let count = items.len();
    if count == 0 {
        return 0;
    }
    let pin = items.iter().filter(|item| !item.pinned).count() >= items.len().div_ceil(2);
    for item in items {
        if item.pinned != pin {
            set_item_pinned_optimistic(item, pin, cx);
        }
    }
    cx.update_global::<ItemSelection, _>(|sel, _| sel.clear());
    count
}
