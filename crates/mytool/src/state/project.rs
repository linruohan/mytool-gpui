use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::ProjectModel;

use crate::{load_projects, state::DBState};

pub struct ProjectState {
    pub projects: Vec<Rc<ProjectModel>>,
}

impl Global for ProjectState {}

impl ProjectState {
    pub fn init(cx: &mut App) {
        let this = ProjectState { projects: vec![] };
        cx.set_global(this);
        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_projects(db.clone()).await;
            let rc_list: Vec<Rc<ProjectModel>> =
                list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("all projects: {}", list.len());
            let _ = cx.update_global::<ProjectState, _>(|state, _cx| {
                state.projects = rc_list;
            });
        })
        .detach();
    }
}
