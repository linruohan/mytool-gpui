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
use gpui::{App, BorrowAppContext};
pub use observer::*;
pub use pending_tasks::*;
use sea_orm::DatabaseConnection;
pub use store::*;
use todos::entity;

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

/// 初始化所有状态
///
/// 新架构使用 TodoStore 作为唯一数据源，
/// 简化代码并消除状态不一致风险。
pub fn state_init(cx: &mut App, db: sea_orm::DatabaseConnection) {
    // 🚀 初始化数据库连接状态（Store 延迟初始化）
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

    // 🚀 初始化观察者注册表（解决过度订阅问题）
    cx.set_global(ObserverRegistry::new());

    // 🚀 初始化脏标记系统
    cx.set_global(DirtyFlags::new());

    // 🚀 初始化待处理任务状态（用于跟踪异步保存操作）
    cx.set_global(PendingTasksState::new());

    // 异步加载数据
    cx.spawn(async move |cx| {
        // 🚀 关键修复：在异步任务中预初始化全局 Store，避免后续在 UI 线程的阻塞调用中初始化导致死锁
        println!("[DEBUG] Pre-initializing global Store in async task...");

        // 加载数据到 TodoStore（唯一数据源）
        println!("[DEBUG] Loading items...");
        let items = crate::state_service::load_items(db.clone()).await;
        println!("[DEBUG] Loaded {} items", items.len());

        // 打印每个项目的 pinned 状态和 due
        for item in &items {
            println!(
                "[DEBUG] Item {}: content={}, pinned={}, due={:?}",
                item.id, item.content, item.pinned, item.due
            );
        }

        // 检查 inbox 条件的任务
        let inbox_items: Vec<&entity::ItemModel> = items
            .iter()
            .filter(|item| item.project_id.is_none() || item.project_id.as_deref() == Some(""))
            .collect();
        println!("[DEBUG] Found {} inbox items (no project ID)", inbox_items.len());

        for (i, item) in inbox_items.iter().enumerate() {
            println!(
                "[DEBUG] Inbox item {}: {}, pinned={}, due={:?}",
                i + 1,
                item.content,
                item.pinned,
                item.due
            );
        }

        println!("[DEBUG] Loading projects...");
        let projects = crate::state_service::load_projects(db.clone()).await;
        println!("[DEBUG] Loaded {} projects", projects.len());

        println!("[DEBUG] Loading sections...");
        let sections = crate::state_service::load_sections(db.clone()).await;
        println!("[DEBUG] Loaded {} sections", sections.len());

        println!("[DEBUG] Loading labels...");
        let labels = crate::state_service::load_labels(db.clone()).await;
        println!("[DEBUG] Loaded {} labels", labels.len());

        // 🚀 关键修复：使用 cx.update 来更新全局状态
        // 注意：需要在 GPUI 的主线程中更新
        cx.update(|cx| {
            println!("[DEBUG] Updating TodoStore in UI thread...");
            cx.update_global::<TodoStore, _>(|store, _| {
                store.set_items(items);
                store.set_projects(projects);
                store.set_sections(sections);
                store.set_labels(labels);
            });
            println!("[DEBUG] TodoStore updated");

            // 发布批量更新事件
            cx.update_global::<TodoEventBus, _>(|bus, _| {
                bus.publish(TodoStoreEvent::BulkUpdate);
            });

            // 🚀 标记所有视图为脏（初始化后需要更新）
            cx.update_global::<DirtyFlags, _>(|flags, _| {
                flags.mark_dirty(ViewType::Inbox);
                flags.mark_dirty(ViewType::Today);
                flags.mark_dirty(ViewType::Scheduled);
                flags.mark_dirty(ViewType::Completed);
                flags.mark_dirty(ViewType::Pinned);
            });
        });
    })
    .detach();
}
