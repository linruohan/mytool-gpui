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
/// PinnedItemState、CompleteItemState），简化代码并消除状态不一致风险。
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

        let arc_items: Vec<_> = items.iter().map(|i| std::sync::Arc::new(i.clone())).collect();

        // 更新 TodoStore
        let _ = cx.update_global::<TodoStore, _>(|store, _| {
            store.set_items(items);
            store.set_projects(projects);
            store.set_sections(sections);
        });

        // 同步更新 ItemState（供 item_row、list_story 等仍使用 ItemState 的组件）
        let _ = cx.update_global::<ItemState, _>(|state, _| {
            state.items = arc_items;
        });

        // 设置数据库连接到全局状态
        let _ = cx.update(|cx| {
            cx.set_global::<DBState>(DBState { conn: db });
        });
    })
    .detach();

    // 初始化其他非Item状态（Project、Label、Section）
    init_other_states(cx);
}

/// 初始化其他非Item状态
///
/// Item数据由 TodoStore 统一维护，这里只初始化 Project、Label、Section 状态
fn init_other_states(cx: &mut App) {
    // 设置空的状态结构
    cx.set_global(ItemState { items: vec![] });
    cx.set_global(ProjectState {
        projects: vec![],
        active_project: None,
        items: vec![],
        sections: vec![],
    });
    cx.set_global(LabelState { labels: vec![] });
    cx.set_global(SectionState { sections: vec![] });

    // 注册监听器
    setup_state_observers(cx);
}

/// 设置状态的监听器
///
/// 仅加载 Project/Label/Section 等非 Item 状态；
/// Item 数据由 TodoStore + refresh_store 统一维护，不再触发 6 个分类查询。
fn setup_state_observers(cx: &mut App) {
    cx.observe_global::<DBState>(|cx| {
        let db = cx.global::<DBState>().conn.clone();
        spawn_load_project_state(db.clone(), cx);
        spawn_load_label_state(db.clone(), cx);
        spawn_load_section_state(db.clone(), cx);
    })
    .detach();
}

// 以下为 Project/Label/Section 状态的加载函数（Item 由 TodoStore 统一维护）

fn spawn_load_project_state(db: sea_orm::DatabaseConnection, cx: &mut App) {
    cx.spawn(async move |cx| {
        let list = crate::state_service::load_projects(db).await;
        let arc_list: Vec<_> = list.iter().map(|item| std::sync::Arc::new(item.clone())).collect();
        let _ = cx.update_global::<ProjectState, _>(|state, _| {
            state.projects = arc_list;
        });
    })
    .detach();
}

fn spawn_load_label_state(db: sea_orm::DatabaseConnection, cx: &mut App) {
    cx.spawn(async move |cx| {
        let list = crate::state_service::load_labels(db).await;
        let arc_list: Vec<_> = list.iter().map(|item| std::sync::Arc::new(item.clone())).collect();
        let _ = cx.update_global::<LabelState, _>(|state, _| {
            state.labels = arc_list;
        });
    })
    .detach();
}

fn spawn_load_section_state(db: sea_orm::DatabaseConnection, cx: &mut App) {
    cx.spawn(async move |cx| {
        let list = crate::state_service::load_sections(db).await;
        let arc_list: Vec<_> = list.iter().map(|item| std::sync::Arc::new(item.clone())).collect();
        let _ = cx.update_global::<SectionState, _>(|state, _| {
            state.sections = arc_list;
        });
    })
    .detach();
}
