use std::sync::Arc;

use gpui::App;
use todos::entity::SectionModel;

use crate::core::state::{TodoStore, get_store};

// Add Section（使用增量更新和全局 Store）
pub fn add_section(section: Arc<SectionModel>, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::add_section_with_store(section.clone(), store).await {
            Ok(new_section) => {
                // 增量更新：只添加新分区到 TodoStore
                let arc_section = Arc::new(new_section);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.add_section(arc_section);
                });
            },
            Err(e) => tracing::error!("add_section failed: {:?}", e),
        }
    })
    .detach();
}

// 修改 section（使用增量更新和全局 Store）
pub fn update_section(section: Arc<SectionModel>, cx: &mut App) {
    let store = get_store(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::mod_section_with_store(section.clone(), store).await {
            Ok(updated_section) => {
                // 增量更新：只更新修改的分区
                let arc_section = Arc::new(updated_section);
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.update_section(arc_section);
                });
            },
            Err(e) => tracing::error!("update_section failed: {:?}", e),
        }
    })
    .detach();
}

// 删除 section（使用增量更新和全局 Store）
pub fn delete_section(section: Arc<SectionModel>, cx: &mut App) {
    let store = get_store(cx);
    let section_id = section.id.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::del_section_with_store(section.clone(), store).await {
            Ok(_) => {
                // 增量更新：只删除指定的分区
                cx.update_global::<TodoStore, _>(|todo_store, _| {
                    todo_store.remove_section(&section_id);
                });
            },
            Err(e) => tracing::error!("delete_section failed: {:?}", e),
        }
    })
    .detach();
}
