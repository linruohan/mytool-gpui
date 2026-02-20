/// 批量操作模块
///
/// 提供批量添加、更新、删除任务的功能，减少数据库 I/O 操作，提升性能。
///
/// 使用场景：
/// - 导入多个任务
/// - 批量修改任务状态
/// - 批量删除任务
/// - 批量完成任务
use std::sync::Arc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::ItemModel;
use tracing::{error, info};

use crate::todo_state::{DBState, TodoStore};

/// 批量操作队列
///
/// 收集待处理的操作，在合适的时机批量提交到数据库
pub struct BatchQueue {
    pub pending_adds: Vec<Arc<ItemModel>>,
    pub pending_updates: Vec<Arc<ItemModel>>,
    pub pending_deletes: Vec<String>,
    pub pending_completes: Vec<(String, bool)>, // (item_id, checked)
}

impl BatchQueue {
    pub fn new() -> Self {
        Self {
            pending_adds: Vec::new(),
            pending_updates: Vec::new(),
            pending_deletes: Vec::new(),
            pending_completes: Vec::new(),
        }
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.pending_adds.is_empty()
            && self.pending_updates.is_empty()
            && self.pending_deletes.is_empty()
            && self.pending_completes.is_empty()
    }

    /// 获取队列中的操作总数
    pub fn total_operations(&self) -> usize {
        self.pending_adds.len()
            + self.pending_updates.len()
            + self.pending_deletes.len()
            + self.pending_completes.len()
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.pending_adds.clear();
        self.pending_updates.clear();
        self.pending_deletes.clear();
        self.pending_completes.clear();
    }
}

impl Default for BatchQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 批量添加任务
///
/// 一次性添加多个任务，减少数据库往返次数
pub fn batch_add_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    if items.is_empty() {
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    let item_count = items.len();

    info!("Batch adding {} items", item_count);

    cx.spawn(async move |cx| {
        let items_vec: Vec<ItemModel> = items.iter().map(|item| (**item).clone()).collect();

        match crate::state_service::batch_add_items(items_vec, db).await {
            Ok(new_items) => {
                info!("Successfully added {} items in batch", new_items.len());

                // 批量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item in new_items {
                        store.add_item(Arc::new(item));
                    }
                });
            },
            Err(e) => {
                error!("Batch add items failed: {:?}", e);
            },
        }
    })
    .detach();
}

/// 批量更新任务
///
/// 一次性更新多个任务，减少数据库往返次数
pub fn batch_update_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    if items.is_empty() {
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    let item_count = items.len();

    info!("Batch updating {} items", item_count);

    cx.spawn(async move |cx| {
        let items_vec: Vec<ItemModel> = items.iter().map(|item| (**item).clone()).collect();

        match crate::state_service::batch_update_items(items_vec, db).await {
            Ok(updated_items) => {
                info!("Successfully updated {} items in batch", updated_items.len());

                // 批量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item in updated_items {
                        store.update_item(Arc::new(item));
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

/// 批量删除任务
///
/// 一次性删除多个任务，减少数据库往返次数
pub fn batch_delete_items(item_ids: Vec<String>, cx: &mut App) {
    if item_ids.is_empty() {
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    let item_count = item_ids.len();
    let ids_clone = item_ids.clone();

    info!("Batch deleting {} items", item_count);

    cx.spawn(async move |cx| {
        match crate::state_service::batch_delete_items(item_ids, db).await {
            Ok(deleted_count) => {
                info!("Successfully deleted {} items in batch", deleted_count);

                // 批量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item_id in ids_clone {
                        store.remove_item(&item_id);
                    }
                });
            },
            Err(e) => {
                error!("Batch delete items failed: {:?}", e);
            },
        }
    })
    .detach();
}

/// 批量完成/取消完成任务
///
/// 一次性修改多个任务的完成状态
pub fn batch_complete_items(item_ids: Vec<String>, checked: bool, cx: &mut App) {
    if item_ids.is_empty() {
        return;
    }

    let db = cx.global::<DBState>().conn.clone();
    let item_count = item_ids.len();
    let ids_clone = item_ids.clone();

    info!("Batch {} {} items", if checked { "completing" } else { "uncompleting" }, item_count);

    cx.spawn(async move |cx| {
        match crate::state_service::batch_complete_items(ids_clone.clone(), checked, false, db)
            .await
        {
            Ok(updated_count) => {
                info!("Successfully updated {} items in batch", updated_count);

                // 批量更新 TodoStore
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item_id in ids_clone {
                        // 查找并更新任务
                        if let Some(item) = store.all_items.iter().find(|i| i.id == item_id) {
                            let mut updated_item = (**item).clone();
                            updated_item.checked = checked;
                            if checked {
                                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                            } else {
                                updated_item.completed_at = None;
                            }
                            store.update_item(Arc::new(updated_item));
                        }
                    }
                });
            },
            Err(e) => {
                error!("Batch complete items failed: {:?}", e);
            },
        }
    })
    .detach();
}

/// 刷新批量操作队列
///
/// 将队列中的所有操作提交到数据库
pub async fn flush_batch_queue(queue: &mut BatchQueue, cx: &mut AsyncApp, db: DatabaseConnection) {
    if queue.is_empty() {
        return;
    }

    let total_ops = queue.total_operations();
    info!("Flushing batch queue with {} operations", total_ops);

    // 批量添加
    if !queue.pending_adds.is_empty() {
        let items: Vec<ItemModel> =
            queue.pending_adds.iter().map(|item| (**item).clone()).collect();
        match crate::state_service::batch_add_items(items, db.clone()).await {
            Ok(new_items) => {
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item in new_items {
                        store.add_item(Arc::new(item));
                    }
                });
            },
            Err(e) => {
                error!("Failed to flush pending adds: {:?}", e);
            },
        }
    }

    // 批量更新
    if !queue.pending_updates.is_empty() {
        let items: Vec<ItemModel> =
            queue.pending_updates.iter().map(|item| (**item).clone()).collect();
        match crate::state_service::batch_update_items(items, db.clone()).await {
            Ok(updated_items) => {
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item in updated_items {
                        store.update_item(Arc::new(item));
                    }
                });
            },
            Err(e) => {
                error!("Failed to flush pending updates: {:?}", e);
            },
        }
    }

    // 批量删除
    if !queue.pending_deletes.is_empty() {
        let ids = queue.pending_deletes.clone();
        match crate::state_service::batch_delete_items(ids.clone(), db.clone()).await {
            Ok(_) => {
                let _ = cx.update_global::<TodoStore, _>(|store, _| {
                    for item_id in ids {
                        store.remove_item(&item_id);
                    }
                });
            },
            Err(e) => {
                error!("Failed to flush pending deletes: {:?}", e);
            },
        }
    }

    // 批量完成
    if !queue.pending_completes.is_empty() {
        // 按 checked 状态分组
        let mut to_complete = Vec::new();
        let mut to_uncomplete = Vec::new();

        for (item_id, checked) in &queue.pending_completes {
            if *checked {
                to_complete.push(item_id.clone());
            } else {
                to_uncomplete.push(item_id.clone());
            }
        }

        // 批量完成
        if !to_complete.is_empty() {
            match crate::state_service::batch_complete_items(
                to_complete.clone(),
                true,
                false,
                db.clone(),
            )
            .await
            {
                Ok(_) => {
                    let _ = cx.update_global::<TodoStore, _>(|store, _| {
                        for item_id in to_complete {
                            if let Some(item) = store.all_items.iter().find(|i| i.id == item_id) {
                                let mut updated_item = (**item).clone();
                                updated_item.checked = true;
                                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                                store.update_item(Arc::new(updated_item));
                            }
                        }
                    });
                },
                Err(e) => {
                    error!("Failed to flush pending completes: {:?}", e);
                },
            }
        }

        // 批量取消完成
        if !to_uncomplete.is_empty() {
            match crate::state_service::batch_complete_items(
                to_uncomplete.clone(),
                false,
                false,
                db.clone(),
            )
            .await
            {
                Ok(_) => {
                    let _ = cx.update_global::<TodoStore, _>(|store, _| {
                        for item_id in to_uncomplete {
                            if let Some(item) = store.all_items.iter().find(|i| i.id == item_id) {
                                let mut updated_item = (**item).clone();
                                updated_item.checked = false;
                                updated_item.completed_at = None;
                                store.update_item(Arc::new(updated_item));
                            }
                        }
                    });
                },
                Err(e) => {
                    error!("Failed to flush pending uncompletes: {:?}", e);
                },
            }
        }
    }

    // 清空队列
    queue.clear();
    info!("Batch queue flushed successfully");
}
