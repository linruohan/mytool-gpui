mod database;
mod item;
mod label;
mod project;
mod section;
mod todo_store;

pub use database::*;
use gpui::App;
pub use item::*;
pub use label::*;
pub use project::*;
pub use section::*;
pub use todo_store::*;
use todos::entity;

/// 初始化所有状态
///
/// 新架构使用 TodoStore 作为唯一数据源，
/// 移除了旧的分散状态（InboxItemState、TodayItemState、ScheduledItemState、
/// PinnedItemState、CompleteItemState、ItemState、ProjectState、LabelState、SectionState），
/// 简化代码并消除状态不一致风险。
pub fn state_init(cx: &mut App, db: sea_orm::DatabaseConnection) {
    // 初始化统一的 TodoStore（唯一数据源）
    cx.set_global(TodoStore::new());

    // 异步加载数据
    cx.spawn(async move |cx| {
        // 加载数据到 TodoStore（唯一数据源）
        println!("[DEBUG] Loading items...");
        let items = crate::state_service::load_items(db.clone()).await;
        println!("[DEBUG] Loaded {} items", items.len());

        // 检查 inbox 条件的任务
        let inbox_items: Vec<&entity::ItemModel> = items
            .iter()
            .filter(|item| item.project_id.is_none() || item.project_id.as_deref() == Some(""))
            .collect();
        println!("[DEBUG] Found {} inbox items (no project ID)", inbox_items.len());

        for (i, item) in inbox_items.iter().enumerate() {
            println!("[DEBUG] Inbox item {}: {}", i + 1, item.content);
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

        // 更新 TodoStore
        println!("[DEBUG] Updating TodoStore...");
        cx.update_global::<TodoStore, _>(|store, _| {
            store.set_items(items);
            store.set_projects(projects);
            store.set_sections(sections);
            store.set_labels(labels);
        });
        println!("[DEBUG] TodoStore updated");
    })
    .detach();
}
