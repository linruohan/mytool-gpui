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
/// ```rust
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
/// # 示例
/// ```rust
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

    // 异步加载数据并预初始化 Store
    cx.spawn(async move |cx| {
        // 使用全局 Store 加载数据
        tracing::info!("Loading data using global Store...");

        // 获取全局 Store 实例
        let store = cx.update_global::<DBState, _>(|state, _| state.get_store());

        // 加载数据到 TodoStore（唯一数据源）
        let items = crate::state_service::load_items_with_store(store.clone()).await;
        tracing::info!("Loaded {} items", items.len());

        // 打印每个项目的 pinned 状态和 due
        // for item in &items {
        //     println!(
        //         "[DEBUG] Item {}: content={}, pinned={}, due={:?}",
        //         item.id, item.content, item.pinned, item.due
        //     );
        // }

        let inbox_items: Vec<&entity::ItemModel> = items
            .iter()
            .filter(|item| item.project_id.is_none() || item.project_id.as_deref() == Some(""))
            .collect();
        tracing::info!("Found {} inbox items (no project ID)", inbox_items.len());

        // for (i, item) in inbox_items.iter().enumerate() {
        //     println!(
        //         "[DEBUG] Inbox item {}: {}, pinned={}, due={:?}",
        //         i + 1,
        //         item.content,
        //         item.pinned,
        //         item.due
        //     );
        // }

        let projects = crate::state_service::load_projects_with_store(store.clone()).await;
        tracing::info!("Loaded {} projects", projects.len());

        let sections = crate::state_service::load_sections_with_store(store.clone()).await;
        tracing::info!("Loaded {} sections", sections.len());

        let labels = crate::state_service::load_labels_with_store(store.clone()).await;
        tracing::info!("Loaded {} labels", labels.len());

        // 将加载的数据更新到 TodoStore
        // 使用 cx.update_global 在异步上下文中安全地更新全局状态
        cx.update_global::<TodoStore, _>(|store, _| {
            tracing::info!(
                "Updating TodoStore with {} items, {} projects, {} sections, {} labels",
                items.len(),
                projects.len(),
                sections.len(),
                labels.len()
            );
            store.set_items(items);
            store.set_projects(projects);
            store.set_sections(sections);
            store.set_labels(labels);
        });

        tracing::info!("TodoStore updated successfully, UI will be notified");
    })
    .detach();
}
