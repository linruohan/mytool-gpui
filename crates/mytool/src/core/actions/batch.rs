//! 批量操作模块
//!
//! 提供批量更新任务的功能，减少数据库 I/O 操作。

use std::sync::Arc;

use gpui::App;
use todos::entity::ItemModel;
use tracing::{error, info};

use crate::core::state::TodoStore;

/// 批量更新任务
pub fn batch_update_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    if items.is_empty() {
        return;
    }

    let item_count = items.len();
    info!("Batch updating {} items", item_count);

    cx.spawn(async move |cx| {
        let store =
            cx.update_global::<crate::core::state::DBState, _>(|state, _| state.get_store());
        let items_vec: Vec<ItemModel> = items.iter().map(|item| (**item).clone()).collect();

        match store.batch_update_items(items_vec).await {
            Ok(updated_items) => {
                info!("Successfully updated {} items in batch", updated_items.len());
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    for item in updated_items {
                        todo_store.update_item(Arc::new(item));
                    }
                });
            },
            Err(e) => {
                error!("Batch update items failed: {:?}", e);
            },
        }
    })
    .detach();
}
