use std::sync::Arc;

use gpui::{App, Global};
use todos::entity::SectionModel;

use crate::{state_service::load_sections, todo_state::DBState};

pub struct SectionState {
    pub sections: Vec<Arc<SectionModel>>,
}

impl Global for SectionState {}

impl SectionState {
    pub fn init(cx: &mut App) {
        let this = SectionState { sections: vec![] };
        cx.set_global(this);

        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let list = load_sections(db.clone()).await;
            let rc_list: Vec<Arc<SectionModel>> =
                list.iter().map(|pro| Arc::new(pro.clone())).collect();
            println!("state sections: {}", list.len());
            cx.update_global::<SectionState, _>(|state, _cx| {
                state.sections = rc_list;
            });
        })
        .detach();
    }
}
