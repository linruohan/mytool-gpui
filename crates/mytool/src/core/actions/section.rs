use std::sync::Arc;

use gpui::App;
use todos::entity::SectionModel;

use crate::core::state::{TodoStore, get_db_connection};

// 添加section（使用增量更新，性能最优）
pub fn add_section(section: Arc<SectionModel>, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::add_section(section.clone(), (*db).clone()).await {
            Ok(new_section) => {
                // 增量更新：只添加新分区到 TodoStore
                let arc_section = Arc::new(new_section);
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_section(arc_section);
                });
            },
            Err(e) => tracing::error!("add_section failed: {:?}", e),
        }
    })
    .detach();
}

// 修改section（使用增量更新，性能最优）
pub fn update_section(section: Arc<SectionModel>, cx: &mut App) {
    let db = get_db_connection(cx);
    cx.spawn(async move |cx| {
        match crate::state_service::mod_section(section.clone(), (*db).clone()).await {
            Ok(updated_section) => {
                // 增量更新：只更新修改的分区
                let arc_section = Arc::new(updated_section);
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_section(arc_section);
                });
            },
            Err(e) => tracing::error!("update_section failed: {:?}", e),
        }
    })
    .detach();
}

// 删除section（使用增量更新，性能最优）
pub fn delete_section(section: Arc<SectionModel>, cx: &mut App) {
    let db = get_db_connection(cx);
    let section_id = section.id.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::del_section(section.clone(), (*db).clone()).await {
            Ok(_) => {
                // 增量更新：只删除指定的分区
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.remove_section(&section_id);
                });
            },
            Err(e) => tracing::error!("delete_section failed: {:?}", e),
        }
    })
    .detach();
}
