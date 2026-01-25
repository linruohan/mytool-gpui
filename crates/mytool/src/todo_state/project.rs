use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::{ItemModel, ProjectModel, SectionModel};

use crate::{
    service::{load_projects, load_sections},
    todo_state::DBState,
};

pub struct ProjectState {
    pub projects: Vec<Rc<ProjectModel>>,
    pub active_project: Option<Rc<ProjectModel>>,
    pub items: Vec<Rc<ItemModel>>,
    pub sections: Vec<Rc<SectionModel>>,
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
        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_projects(db.clone()).await;
            let rc_list: Vec<Rc<ProjectModel>> =
                list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state projects: {}", list.len());
            cx.update_global::<ProjectState, _>(|state, _cx| {
                state.projects = rc_list;
            });
        })
        .detach();

        // 加载sections
        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_sections(db.clone()).await;
            let rc_list: Vec<Rc<SectionModel>> =
                list.iter().map(|sec| Rc::new(sec.clone())).collect();
            println!("state project sections: {}", list.len());
            cx.update_global::<ProjectState, _>(|state, _cx| {
                state.sections = rc_list;
            });
        })
        .detach();
    }
}
