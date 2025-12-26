use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::SectionModel;

use crate::{service::load_sections, todo_state::DBState};

#[derive(Clone, PartialEq)]
pub enum SectionStatus {
    Added,
    Modified,
    Deleted,
    Loaded,
}

pub struct SectionState {
    pub sections: Vec<Rc<SectionModel>>,
    active_section: Option<Rc<SectionModel>>,
}

impl Global for SectionState {}

impl SectionState {
    pub fn init(cx: &mut App) {
        let this = SectionState { sections: vec![], active_section: None };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_sections(db.clone()).await;
            let rc_list: Vec<Rc<SectionModel>> =
                list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("all items: {}", list.len());
            let _ = cx.update_global::<SectionState, _>(|state, _cx| {
                state.sections = rc_list;
            });
        })
        .detach();
    }

    pub fn set_sections(&mut self, sections: impl IntoIterator<Item = SectionModel>) {
        self.sections = sections.into_iter().map(Rc::new).collect();
        self.active_section = None;
    }
}
