use std::sync::Arc;

use gpui::{App, Global};
use todos::entity::{ItemModel, ProjectModel, SectionModel};

use crate::{
    state_service::{get_items_by_project_id, load_projects, load_sections},
    todo_state::DBState,
};

pub struct ProjectState {
    pub projects: Vec<Arc<ProjectModel>>,
    pub active_project: Option<Arc<ProjectModel>>,
    pub items: Vec<Arc<ItemModel>>,
    pub sections: Vec<Arc<SectionModel>>,
}

impl Global for ProjectState {}

impl ProjectState {
    pub fn init(cx: &mut App) {
        let this = ProjectState {
            projects: vec![],
            active_project: None,
            items: vec![],
            sections: vec![],
        };
        cx.set_global(this);
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = load_projects(db.clone()).await;
            let rc_list: Vec<Arc<ProjectModel>> =
                list.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state projects: {}", list.len());
            cx.update_global::<ProjectState, _>(|state, _cx| {
                state.projects = rc_list;
            });
        })
        .detach();

        // 加载sections
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = load_sections(db.clone()).await;
            let rc_list: Vec<Arc<SectionModel>> =
                list.iter().map(|sec| Arc::new(sec.clone())).collect();
            println!("state project sections: {}", list.len());
            cx.update_global::<ProjectState, _>(|state, _cx| {
                state.sections = rc_list;
            });
        })
        .detach();

        // 订阅ItemState的变化，当ItemState改变时更新ProjectState中的items
        cx.observe_global::<crate::todo_state::ItemState>(move |cx| {
            let db = cx.global::<DBState>().conn.clone();
            let active_project = cx.global::<ProjectState>().active_project.clone();
            cx.spawn(async move |cx| {
                if let Some(active_project) = active_project {
                    let list = get_items_by_project_id(&active_project.id, db.clone()).await;
                    let rc_list: Vec<Arc<ItemModel>> =
                        list.iter().map(|item| Arc::new(item.clone())).collect();
                    println!("state project items updated: {}", list.len());
                    cx.update_global::<ProjectState, _>(|state, _cx| {
                        state.items = rc_list;
                    });
                }
            })
            .detach();
        })
        .detach();
    }
}
