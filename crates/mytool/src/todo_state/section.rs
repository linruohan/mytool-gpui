use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::SectionModel;

use crate::{service::load_sections, todo_state::DBState};

pub struct SectionState {
    pub sections: Vec<Rc<SectionModel>>,
}

impl Global for SectionState {}

impl SectionState {
    pub fn init(cx: &mut App) {
        let this = SectionState { sections: vec![] };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_sections(db.clone()).await;
            let rc_list: Vec<Rc<SectionModel>> =
                list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("state sections: {}", list.len());
            cx.update_global::<SectionState, _>(|state, _cx| {
                state.sections = rc_list;
            });
        })
        .detach();
    }
}
