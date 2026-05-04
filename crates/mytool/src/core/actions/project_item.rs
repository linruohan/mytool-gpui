use std::sync::Arc;

use gpui::{App, AsyncApp, BorrowAppContext};
use todos::entity::{ItemModel, ProjectModel};

use crate::core::state::TodoStore;

pub fn load_project_items(project: Arc<ProjectModel>, cx: &mut App) {
    tracing::debug!(
        "load_project_items 开始，project_id: {}, project_name: {}",
        project.id,
        project.name
    );

    // 检查 project_id 是否有效
    if project.id.is_empty() {
        tracing::error!("load_project_items: project_id 为空，跳过加载");
        return;
    }

    // 记录当前激活的 project，供异步刷新时做竞态保护
    cx.update_global::<TodoStore, _>(|state, _| {
        state.set_active_project(Some(project.clone()));
    });

    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        // 在异步任务内部获取 store，避免多次移动
        refresh_project_items_impl(&project_id, cx).await;
    })
    .detach();
}

// 刷新 items
async fn refresh_project_items_impl(project_id: &str, cx: &mut AsyncApp) {
    tracing::debug!("开始刷新项目 items, project_id: {}", project_id);

    // 检查 project_id 是否有效
    if project_id.is_empty() {
        tracing::error!("project_id 为空，跳过刷新");
        return;
    }

    // 在异步任务内部获取 store
    let store = cx.update_global::<crate::core::state::DBState, _>(|state, _| state.get_store());

    // 获取项目 items
    let items = crate::state_service::get_items_by_project_id_with_store(project_id, store).await;
    let arc_items: Vec<Arc<ItemModel>> = items.iter().map(|item| Arc::new(item.clone())).collect();
    tracing::debug!("成功加载项目 items: {} 个", arc_items.len());

    // 只在当前激活项目仍然是该 project_id 时更新，避免快速切换导致旧请求覆盖新项目的 items
    cx.update_global::<TodoStore, _>(|state, _| {
        if let Some(active) = &state.active_project
            && active.id == project_id
        {
            // 使用增量更新：先移除旧 items，再添加新 items
            // 注意：这里使用批量更新方式，因为 TodoStore 的 items 是全局的
            // 我们需要先移除属于该项目的所有 items，再添加新的 items
            state.all_items.retain(|item| item.project_id.as_deref() != Some(project_id));
            for item in arc_items.iter() {
                state.add_item(item.clone());
            }
            tracing::debug!("已更新 TodoStore.items, 数量：{}", arc_items.len());
        } else {
            tracing::debug!("激活项目已变更，跳过更新");
        }
    });
}

// 添加 item
pub fn add_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        let store =
            cx.update_global::<crate::core::state::DBState, _>(|state, _| state.get_store());
        if crate::state_service::add_item_with_store(item.clone(), store).await.is_ok() {
            refresh_project_items_impl(&project_id, cx).await;
        }
    })
    .detach();
}

// 修改 item
pub fn update_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        let store =
            cx.update_global::<crate::core::state::DBState, _>(|state, _| state.get_store());
        if crate::state_service::mod_item_with_store(item.clone(), store).await.is_ok() {
            refresh_project_items_impl(&project_id, cx).await;
        }
    })
    .detach();
}

// 删除 item
pub fn delete_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        let store =
            cx.update_global::<crate::core::state::DBState, _>(|state, _| state.get_store());
        if crate::state_service::del_item_with_store(item.clone(), store).await.is_ok() {
            refresh_project_items_impl(&project_id, cx).await;
        }
    })
    .detach();
}
