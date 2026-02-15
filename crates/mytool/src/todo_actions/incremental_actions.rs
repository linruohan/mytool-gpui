//! 增量更新操作模块
//!
//! 这个模块提供了对 TodoStore 的增量更新操作接口。
//! 与 store_actions.rs 中的全量刷新不同，这里只更新单条数据，大幅提升性能。
//!
//! 使用场景：
//! - 添加/更新/删除单个任务时，只更新该任务，不刷新全部数据
//! - 预期可减少 90%+ 的数据传输量

use std::sync::Arc;

use gpui::AsyncApp;
use sea_orm::DatabaseConnection;
use todos::entity::{ItemModel, ProjectModel, SectionModel};

use crate::todo_state::TodoStore;

// ==================== 任务(Item)增量操作 ====================

/// 增量添加任务
///
/// 只将新任务添加到 TodoStore，不刷新全部数据
///
/// # 示例
/// ```rust
/// let item = Arc::new(ItemModel::default());
/// add_item_incremental(item, &mut cx, db).await;
/// ```
pub async fn add_item_incremental(item: Arc<ItemModel>, cx: &mut AsyncApp, db: DatabaseConnection) {
    match crate::state_service::add_item(item.clone(), db).await {
        Ok(new_item) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.add_item(Arc::new(new_item));
            });
        },
        Err(e) => {
            tracing::error!("add_item_incremental failed: {:?}", e);
        },
    }
}

/// 增量更新任务
///
/// 只更新 TodoStore 中对应的单条任务
///
/// # 示例
/// ```rust
/// let item = Arc::new(updated_item);
/// update_item_incremental(item, &mut cx, db).await;
/// ```
pub async fn update_item_incremental(
    item: Arc<ItemModel>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::mod_item(item.clone(), db).await {
        Ok(updated_item) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("update_item_incremental failed: {:?}", e);
        },
    }
}

/// 增量删除任务
///
/// 只从 TodoStore 中移除对应的单条任务
///
/// # 示例
/// ```rust
/// delete_item_incremental("item_id".to_string(), &mut cx, db).await;
/// ```
pub async fn delete_item_incremental(item_id: String, cx: &mut AsyncApp, db: DatabaseConnection) {
    match crate::state_service::del_item_by_id(&item_id, db).await {
        Ok(_) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.remove_item(&item_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_item_incremental failed: {:?}", e);
        },
    }
}

/// 完成任务（增量更新）
///
/// 只更新任务的完成状态，不刷新全部数据
///
/// # 示例
/// ```rust
/// complete_item_incremental(item, true, &mut cx, db).await;
/// ```
pub async fn complete_item_incremental(
    item: Arc<ItemModel>,
    checked: bool,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::finish_item(item.clone(), checked, false, db).await {
        Ok(_) => {
            // 更新本地状态
            let mut updated_item = (*item).clone();
            updated_item.checked = checked;
            if checked {
                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
            } else {
                updated_item.completed_at = None;
            }
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("complete_item_incremental failed: {:?}", e);
        },
    }
}

/// 置顶/取消置顶任务（增量更新）
///
/// # 示例
/// ```rust
/// pin_item_incremental(item, true, &mut cx, db).await;
/// ```
pub async fn pin_item_incremental(
    item: Arc<ItemModel>,
    pinned: bool,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::pin_item(item.clone(), pinned, db).await {
        Ok(_) => {
            // 更新本地状态
            let mut updated_item = (*item).clone();
            updated_item.pinned = pinned;
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("pin_item_incremental failed: {:?}", e);
        },
    }
}

// ==================== 项目(Project)增量操作 ====================

/// 增量添加项目
pub async fn add_project_incremental(
    project: Arc<ProjectModel>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::add_project(project.clone(), db).await {
        Ok(new_project) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.add_project(Arc::new(new_project));
            });
        },
        Err(e) => {
            tracing::error!("add_project_incremental failed: {:?}", e);
        },
    }
}

/// 增量更新项目
pub async fn update_project_incremental(
    project: Arc<ProjectModel>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::mod_project(project.clone(), db).await {
        Ok(updated_project) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.update_project(Arc::new(updated_project));
            });
        },
        Err(e) => {
            tracing::error!("update_project_incremental failed: {:?}", e);
        },
    }
}

/// 增量删除项目
pub async fn delete_project_incremental(
    project_id: String,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::del_project_by_id(&project_id, db).await {
        Ok(_) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.remove_project(&project_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_project_incremental failed: {:?}", e);
        },
    }
}

// ==================== 分区(Section)增量操作 ====================

/// 增量添加分区
pub async fn add_section_incremental(
    section: Arc<SectionModel>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::add_section(section.clone(), db).await {
        Ok(new_section) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.add_section(Arc::new(new_section));
            });
        },
        Err(e) => {
            tracing::error!("add_section_incremental failed: {:?}", e);
        },
    }
}

/// 增量更新分区
pub async fn update_section_incremental(
    section: Arc<SectionModel>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::mod_section(section.clone(), db).await {
        Ok(updated_section) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.update_section(Arc::new(updated_section));
            });
        },
        Err(e) => {
            tracing::error!("update_section_incremental failed: {:?}", e);
        },
    }
}

/// 增量删除分区
pub async fn delete_section_incremental(
    section_id: String,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    match crate::state_service::del_section_by_id(&section_id, db).await {
        Ok(_) => {
            let _ = cx.update_global::<TodoStore, _>(|store, _| {
                store.remove_section(&section_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_section_incremental failed: {:?}", e);
        },
    }
}

// ==================== 批量操作 ====================

/// 批量删除任务（增量更新）
///
/// 用于批量删除场景，如清空已完成任务
pub async fn batch_delete_items_incremental(
    item_ids: Vec<String>,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    let mut success_count = 0;

    for item_id in &item_ids {
        match crate::state_service::del_item_by_id(item_id, db.clone()).await {
            Ok(_) => success_count += 1,
            Err(e) => tracing::error!("batch_delete_items failed for {}: {:?}", item_id, e),
        }
    }

    let _ = cx.update_global::<TodoStore, _>(|store, _| {
        for item_id in &item_ids {
            store.remove_item(item_id);
        }
    });

    tracing::info!("batch_delete_items completed: {}/{} succeeded", success_count, item_ids.len());
}

/// 批量完成任务（增量更新）
///
/// 用于批量完成场景，如完成所有过期任务
pub async fn batch_complete_items_incremental(
    items: Vec<Arc<ItemModel>>,
    checked: bool,
    cx: &mut AsyncApp,
    db: DatabaseConnection,
) {
    let mut updated_items = Vec::new();

    for item in &items {
        match crate::state_service::finish_item(item.clone(), checked, false, db.clone()).await {
            Ok(_) => {
                // 更新本地状态
                let mut updated_item = (**item).clone();
                updated_item.checked = checked;
                if checked {
                    updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
                } else {
                    updated_item.completed_at = None;
                }
                updated_items.push(Arc::new(updated_item));
            },
            Err(e) => tracing::error!("batch_complete_items failed for {}: {:?}", item.id, e),
        }
    }

    let _ = cx.update_global::<TodoStore, _>(|store, _| {
        for item in updated_items {
            store.update_item(item);
        }
    });

    tracing::info!("batch_complete_items completed: {} items updated", items.len());
}
