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

/// 初始化所有状态
///
/// 新架构使用 TodoStore 作为唯一数据源，
/// 移除了旧的分散状态（InboxItemState、TodayItemState、ScheduledItemState、
/// PinnedItemState、CompleteItemState、ItemState、ProjectState、LabelState、SectionState），
/// 简化代码并消除状态不一致风险。
pub fn state_init(cx: &mut App) {
    // 初始化统一的 TodoStore（唯一数据源）
    cx.set_global(TodoStore::new());

    // 异步初始化数据库连接并加载数据
    cx.spawn(async move |cx| {
        // 初始化数据库连接
        let db = get_todo_conn().await;

        // 加载数据到 TodoStore（唯一数据源）
        let items = crate::state_service::load_items(db.clone()).await;
        let projects = crate::state_service::load_projects(db.clone()).await;
        let sections = crate::state_service::load_sections(db.clone()).await;
        let labels = crate::state_service::load_labels(db.clone()).await;

        // 更新 TodoStore
        cx.update_global::<TodoStore, _>(|store, _| {
            store.set_items(items);
            store.set_projects(projects);
            store.set_sections(sections);
            store.set_labels(labels);
        });

        // 设置数据库连接到全局状态
        cx.update(|cx| {
            cx.set_global::<DBState>(DBState { conn: db });
        });
    })
    .detach();
}
