//! 批量操作模块
//!
//! 提供批量更新任务的功能，减少数据库 I/O 操作。

use std::sync::Arc;

use gpui::App;
use todos::entity::ItemModel;
use tracing::{debug, error};

use crate::{core::state::TodoStore, todo_state::DBState};

/// 批量更新任务
pub fn batch_update_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    if items.is_empty() {
        return;
    }

    let item_count = items.len();
    debug!("Batch updating {} items", item_count);

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
