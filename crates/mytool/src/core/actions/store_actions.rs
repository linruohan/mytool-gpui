//! TodoStore 操作模块
//!
//! 这个模块提供了对 TodoStore 的统一操作接口。
//! 所有操作只更新 TodoStore，然后由 TodoStore 自动派发更新到各个视图。
//!
//! 注意：这些函数使用增量更新，性能最优。

#![allow(dead_code)]

use std::sync::Arc;

use gpui::AsyncApp;
use todos::entity::{ItemModel, ProjectModel, SectionModel};

use crate::todo_state::TodoStore;

/// 刷新 TodoStore（全量刷新）
///
/// 一次性加载所有数据并更新 TodoStore。
/// 注意：这个函数会触发全量查询，性能较低。建议优先使用增量更新函数。
///
/// 适用场景：
/// - 初始加载数据
/// - 数据同步后需要全量刷新
/// - 不确定数据变化范围时
pub async fn refresh_store(cx: &mut AsyncApp, _db: sea_orm::DatabaseConnection) {
    // 获取全局 Store
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    // 一次性加载所有数据
    let items = crate::state_service::load_items_with_store(store.clone()).await;
    let projects = crate::state_service::load_projects_with_store(store.clone()).await;
    let sections = crate::state_service::load_sections_with_store(store.clone()).await;

    // 更新 TodoStore（唯一数据源）
    cx.update_global::<TodoStore, _>(|todo_store, _| {
        todo_store.set_items(items);
        todo_store.set_projects(projects);
        todo_store.set_sections(sections);
    });
}

// ==================== 任务 (Item) 操作 - 增量更新 ====================

/// 添加任务（增量更新）
///
/// 只将新任务添加到 TodoStore，不刷新全部数据
pub async fn add_item_to_store(
    item: Arc<ItemModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::add_item_with_store(item, store).await {
        Ok(new_item) => {
            // 增量更新：只添加新任务
            cx.update_global::<TodoStore, _>(|store, _| {
                store.add_item(Arc::new(new_item));
            });
        },
        Err(e) => {
            tracing::error!("add_item_to_store failed: {:?}", e);
        },
    }
}

/// 更新任务（增量更新）
///
/// 只更新 TodoStore 中对应的单条任务
pub async fn update_item_in_store(
    item: Arc<ItemModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::mod_item_with_store(item, store).await {
        Ok(updated_item) => {
            // 增量更新：只更新修改的任务
            cx.update_global::<TodoStore, _>(|store, _| {
                store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("update_item_in_store failed: {:?}", e);
        },
    }
}

/// 删除任务（增量更新）
///
/// 只从 TodoStore 中移除对应的单条任务
pub async fn delete_item_from_store(
    item: Arc<ItemModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());
    let item_id = item.id.clone();

    match crate::state_service::del_item_with_store(item, store).await {
        Ok(_) => {
            // 增量更新：只删除指定的任务
            cx.update_global::<TodoStore, _>(|store, _| {
                store.remove_item(&item_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_item_from_store failed: {:?}", e);
        },
    }
}

/// 完成任务（增量更新）
///
/// 只更新任务的完成状态，不刷新全部数据
pub async fn complete_item_in_store(
    item: Arc<ItemModel>,
    checked: bool,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::finish_item_with_store(item.clone(), checked, false, store).await {
        Ok(_) => {
            // 增量更新：更新本地状态
            let mut updated_item = (*item).clone();
            updated_item.checked = checked;
            if checked {
                updated_item.completed_at = Some(chrono::Utc::now().naive_utc());
            } else {
                updated_item.completed_at = None;
            }
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("complete_item_in_store failed: {:?}", e);
        },
    }
}

/// 置顶任务（增量更新）
///
/// 只更新任务的置顶状态，不刷新全部数据
pub async fn pin_item_in_store(
    item: Arc<ItemModel>,
    pinned: bool,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::pin_item_with_store(item.clone(), pinned, store).await {
        Ok(_) => {
            // 增量更新：更新本地状态
            let mut updated_item = (*item).clone();
            updated_item.pinned = pinned;
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.update_item(Arc::new(updated_item));
            });
        },
        Err(e) => {
            tracing::error!("pin_item_in_store failed: {:?}", e);
        },
    }
}

// ==================== 项目 (Project) 操作 - 增量更新 ====================

/// 添加项目（增量更新）
pub async fn add_project_to_store(
    project: Arc<ProjectModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::add_project_with_store(project, store).await {
        Ok(new_project) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.add_project(Arc::new(new_project));
            });
        },
        Err(e) => {
            tracing::error!("add_project_to_store failed: {:?}", e);
        },
    }
}

/// 更新项目（增量更新）
pub async fn update_project_in_store(
    project: Arc<ProjectModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::mod_project_with_store(project, store).await {
        Ok(updated_project) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.update_project(Arc::new(updated_project));
            });
        },
        Err(e) => {
            tracing::error!("update_project_in_store failed: {:?}", e);
        },
    }
}

/// 删除项目（增量更新）
pub async fn delete_project_from_store(
    project: Arc<ProjectModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());
    let project_id = project.id.clone();

    match crate::state_service::del_project_with_store(project, store).await {
        Ok(_) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.remove_project(&project_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_project_from_store failed: {:?}", e);
        },
    }
}

// ==================== 分区 (Section) 操作 - 增量更新 ====================

/// 添加分区（增量更新）
pub async fn add_section_to_store(
    section: Arc<SectionModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::add_section_with_store(section, store).await {
        Ok(new_section) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.add_section(Arc::new(new_section));
            });
        },
        Err(e) => {
            tracing::error!("add_section_to_store failed: {:?}", e);
        },
    }
}

/// 更新分区（增量更新）
pub async fn update_section_in_store(
    section: Arc<SectionModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());

    match crate::state_service::mod_section_with_store(section, store).await {
        Ok(updated_section) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.update_section(Arc::new(updated_section));
            });
        },
        Err(e) => {
            tracing::error!("update_section_in_store failed: {:?}", e);
        },
    }
}

/// 删除分区（增量更新）
pub async fn delete_section_from_store(
    section: Arc<SectionModel>,
    cx: &mut AsyncApp,
    _db: sea_orm::DatabaseConnection,
) {
    let store = cx.update_global::<crate::todo_state::DBState, _>(|state, _| state.get_store());
    let section_id = section.id.clone();

    match crate::state_service::del_section_with_store(section, store).await {
        Ok(_) => {
            cx.update_global::<TodoStore, _>(|todo_store, _| {
                todo_store.remove_section(&section_id);
            });
        },
        Err(e) => {
            tracing::error!("delete_section_from_store failed: {:?}", e);
        },
    }
}
