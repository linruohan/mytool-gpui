mod cache;
mod database;
mod events;
mod pending_tasks;
mod store;

pub use cache::*;
pub use database::DBState;
pub use events::*;
use gpui::App;
pub use pending_tasks::*;
use sea_orm::DatabaseConnection;
pub use store::*;
use tracing::error;

/// 获取数据库连接的便捷函数
///
/// # Returns
/// 返回 Result 类型，允许调用者处理错误
pub async fn get_todo_conn() -> Result<DatabaseConnection, sea_orm::DbErr> {
    todos::init_db().await
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

    // 初始化查询缓存
    cx.set_global(QueryCache::new());

    // 初始化错误通知器
    cx.set_global(ErrorNotifier::new());

    // 到期提醒（由 watcher 写入，StoryRoot 弹出）
    cx.set_global(ReminderNotifier::new());

    cx.set_global(ItemSelection::new());

    // 初始化待处理任务状态（用于跟踪异步保存操作）
    cx.set_global(PendingTasksState::new());

    // 初始化保存结果状态
    cx.set_global(SaveResults::new());

    // 异步创建 Store 并加载数据
    cx.spawn(async move |cx| {
        tracing::info!("Initializing Store asynchronously...");

        // 只读克隆，不要 update_global，否则会误通知 DBState 观察者
        let db_state = cx.read_global::<DBState, _>(|db_state, _| db_state.clone());
        let store = db_state
            .init_store()
            .await
            .unwrap_or_else(|e| panic!("Failed to initialize Store: {e}"));

        tracing::info!("Store initialized, loading data...");

        // 并行冷加载：items / projects / sections / labels
        tracing::info!("Loading items, projects, sections, labels in parallel...");
        let (items_r, projects_r, sections_r, labels_r) = tokio::join!(
            store.get_all_items(),
            store.get_all_projects(),
            store.get_all_sections(),
            store.get_all_labels(),
        );

        if let Ok(ref items) = items_r {
            tracing::info!("Loaded {} items", items.len());
        }
        if let Ok(ref projects) = projects_r {
            tracing::info!("Loaded {} projects", projects.len());
        }
        if let Ok(ref sections) = sections_r {
            tracing::info!("Loaded {} sections", sections.len());
        }
        if let Ok(ref labels) = labels_r {
            tracing::info!("Loaded {} labels", labels.len());
        }

        let mut load_failures: Vec<String> = Vec::new();
        if let Err(ref e) = items_r {
            error!(error = %e, "get_all_items failed during startup");
            load_failures.push(format!("任务加载失败: {e}"));
        }
        if let Err(ref e) = projects_r {
            error!(error = %e, "get_all_projects failed during startup");
            load_failures.push(format!("项目加载失败: {e}"));
        }
        if let Err(ref e) = sections_r {
            error!(error = %e, "get_all_sections failed during startup");
            load_failures.push(format!("分区加载失败: {e}"));
        }
        if let Err(ref e) = labels_r {
            error!(error = %e, "get_all_labels failed during startup");
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

    crate::todo_actions::start_reminder_watcher(cx);
}
