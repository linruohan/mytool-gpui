mod cache;
mod database;
mod events;
mod observer;
mod pending_tasks;
mod store;

use std::sync::Arc;

pub use cache::*;
pub use database::*;
pub use events::*;
use gpui::App;
pub use observer::*;
pub use pending_tasks::*;
use sea_orm::DatabaseConnection;
pub use store::*;
use todos::entity;
use tracing::error;

/// 获取数据库连接的便捷函数
///
/// # Returns
/// 返回 Result 类型，允许调用者处理错误
pub async fn get_todo_conn() -> Result<DatabaseConnection, sea_orm::DbErr> {
    todos::init_db().await
}

/// 获取数据库连接的便捷函数
///
/// 这是一个辅助函数，用于简化从全局状态获取数据库连接的操作。
/// 返回的 Arc<DatabaseConnection> 是轻量级的，可以安全地克隆。
///
/// # 示例
/// ```ignore
/// let db = get_db_connection(cx);
/// cx.spawn(async move |cx| {
///     // 使用 db 进行数据库操作
/// })
/// .detach();
/// ```
#[inline]
pub fn get_db_connection(cx: &App) -> Arc<DatabaseConnection> {
    cx.global::<DBState>().get_connection()
}

/// 获取全局 Store 实例的便捷函数
///
/// 这是一个辅助函数，用于简化从全局状态获取 Store 实例的操作。
/// 返回的 Arc<Store> 是轻量级的，可以安全地克隆。
///
/// 🚀 6.1优化：Store 通过异步创建，此方法在 Store 未就绪时会 panic
///
/// # Panics
/// 如果 Store 尚未初始化（这表示应用逻辑有错误）
///
/// # 示例
/// ```ignore
/// let store = get_store(cx);
/// cx.spawn(async move |_cx| {
///     // 使用 store 进行数据库操作
/// })
/// .detach();
/// ```
#[inline]
pub fn get_store(cx: &App) -> Arc<todos::Store> {
    cx.global::<DBState>().get_store()
}

/// 初始化所有状态
///
/// 新架构使用 TodoStore 作为唯一数据源，
/// 简化代码并消除状态不一致风险。
pub fn state_init(cx: &mut App, db: sea_orm::DatabaseConnection) {
    // 初始化数据库连接状态
    cx.set_global(DBState::new(db.clone()));

    // 初始化统一的 TodoStore（唯一数据源）
    cx.set_global(TodoStore::new());

    // 初始化事件总线
    cx.set_global(TodoEventBus::new());

    // 初始化查询缓存
    cx.set_global(QueryCache::new());

    // 初始化批量操作队列
    cx.set_global(BatchOperations::new());

    // 初始化错误通知器
    cx.set_global(ErrorNotifier::new());

    // 初始化观察者注册表
    cx.set_global(ObserverRegistry::new());

    // 初始化脏标记系统
    cx.set_global(DirtyFlags::new());

    // 初始化待处理任务状态（用于跟踪异步保存操作）
    cx.set_global(PendingTasksState::new());

    // 异步创建 Store 并加载数据
    cx.spawn(async move |cx| {
        tracing::info!("Initializing Store asynchronously...");

        // 🚀 6.1优化：异步创建 Store，不阻塞首帧
        // 通过 update_global 获取 DBState 克隆，然后在 async 块中初始化
        let db_state = cx.update_global::<DBState, _>(|db_state, _| db_state.clone());
        let store = db_state
            .init_store()
            .await
            .unwrap_or_else(|e| panic!("Failed to initialize Store: {e}"));

        tracing::info!("Store initialized, loading data...");

        // 加载数据到 TodoStore（唯一数据源）
        let items_r = crate::state_service::load_items_with_store(store.clone()).await;
        if let Ok(ref items) = items_r {
            let inbox_items: Vec<&entity::ItemModel> = items
                .iter()
                .filter(|item| item.project_id.is_none() || item.project_id.as_deref() == Some(""))
                .collect();
            tracing::info!(
                "Loaded {} items ({} inbox without project)",
                items.len(),
                inbox_items.len()
            );
        }

        let projects_r = crate::state_service::load_projects_with_store(store.clone()).await;
        if let Ok(ref projects) = projects_r {
            tracing::info!("Loaded {} projects", projects.len());
        }

        let sections_r = crate::state_service::load_sections_with_store(store.clone()).await;
        if let Ok(ref sections) = sections_r {
            tracing::info!("Loaded {} sections", sections.len());
        }

        let labels_r = crate::state_service::load_labels_with_store(store.clone()).await;
        if let Ok(ref labels) = labels_r {
            tracing::info!("Loaded {} labels", labels.len());
        }

        let mut load_failures: Vec<String> = Vec::new();
        if let Err(ref e) = items_r {
            error!(error = %e, "load_items_with_store failed during startup");
            load_failures.push(format!("任务加载失败: {e}"));
        }
        if let Err(ref e) = projects_r {
            error!(error = %e, "load_projects_with_store failed during startup");
            load_failures.push(format!("项目加载失败: {e}"));
        }
        if let Err(ref e) = sections_r {
            error!(error = %e, "load_sections_with_store failed during startup");
            load_failures.push(format!("分区加载失败: {e}"));
        }
        if let Err(ref e) = labels_r {
            error!(error = %e, "load_labels_with_store failed during startup");
            load_failures.push(format!("标签加载失败: {e}"));
        }

        // 仅应用成功的查询，避免把失败误呈现为「空列表」
        cx.update_global::<TodoStore, _>(|todo_store, _| {
            if let Ok(items) = items_r {
                todo_store.set_items(items);
            }
            if let Ok(projects) = projects_r {
                todo_store.set_projects(projects);
            }
            if let Ok(sections) = sections_r {
                todo_store.set_sections(sections);
            }
            if let Ok(labels) = labels_r {
                todo_store.set_labels(labels);
            }
            tracing::info!("TodoStore cold-load apply finished (partial if any query failed)");
        });

        if !load_failures.is_empty() {
            let msg = load_failures.join(" ");
            cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                notifier.set_error(msg.clone());
            });
        }

        tracing::info!("Initial data load task finished, UI will be notified");
    })
    .detach();
}
