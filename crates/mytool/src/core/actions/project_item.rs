use std::sync::Arc;

use gpui::{App, AsyncApp, BorrowAppContext};
use sea_orm::DatabaseConnection;
use todos::entity::{ItemModel, ProjectModel};

use crate::todo_state::{DBState, TodoStore};

pub fn load_project_items(project: Arc<ProjectModel>, cx: &mut App) {
    println!(
        "[DEBUG] load_project_items 开始, project_id: {}, project_name: {}",
        project.id, project.name
    );

    // 检查 project_id 是否有效
    if project.id.is_empty() {
        println!("[ERROR] load_project_items: project_id 为空,跳过加载");
        return;
    }

    // 记录当前激活的 project,供异步刷新时做竞态保护
    cx.update_global::<TodoStore, _>(|state, _| {
        state.set_active_project(Some(project.clone()));
        println!("[DEBUG] 已更新 TodoStore.active_project: {}", project.name);
    });

    let db = cx.global::<DBState>().conn.clone();
    let project_id = project.id.clone();
    cx.spawn(async move |cx| {
        println!("[DEBUG] 异步任务开始, project_id: {}", project_id);
        refresh_project_items(&project_id, cx, (*db).clone()).await;
        println!("[DEBUG] 异步任务完成, project_id: {}", project_id);
    })
    .detach();
}
// 刷新items
async fn refresh_project_items(project_id: &str, cx: &mut AsyncApp, db: DatabaseConnection) {
    println!("[DEBUG] 开始刷新项目 items, project_id: {}", project_id);

    // 检查 project_id 是否有效
    if project_id.is_empty() {
        println!("[ERROR] project_id 为空,跳过刷新");
        return;
    }

    // 获取项目 items
    let items = crate::state_service::get_items_by_project_id(project_id, db).await;
    let arc_items: Vec<Arc<ItemModel>> = items.iter().map(|item| Arc::new(item.clone())).collect();
    println!("[DEBUG] 成功加载项目 items: {} 个", arc_items.len());

    // 只在当前激活项目仍然是该 project_id 时更新,避免快速切换导致旧请求覆盖新项目的 items
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
            println!("[DEBUG] 已更新 TodoStore.items, 数量: {}", arc_items.len());
        } else {
            println!("[DEBUG] 激活项目已变更,跳过更新");
        }
    });
}
// 添加item
pub fn add_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if crate::state_service::add_item(item.clone(), (*db).clone()).await.is_ok() {
            refresh_project_items(&project.id.clone(), cx, (*db).clone()).await;
        }
    })
    .detach();
}
// 修改item
pub fn update_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if crate::state_service::mod_item(item.clone(), (*db).clone()).await.is_ok() {
            refresh_project_items(&project.id.clone(), cx, (*db).clone()).await;
        }
    })
    .detach();
}
// 删除item
pub fn delete_project_item(project: Arc<ProjectModel>, item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if let Ok(_store) = crate::state_service::del_item(item.clone(), (*db).clone()).await {
            refresh_project_items(&project.id.clone(), cx, (*db).clone()).await;
        }
    })
    .detach();
}
