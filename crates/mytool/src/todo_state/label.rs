use std::rc::Rc;

use gpui::{App, Global};
use todos::entity::LabelModel;

use crate::{service::load_labels, todo_state::DBState};

pub struct LabelState {
    pub labels: Vec<Rc<LabelModel>>,
    active_label: Option<Rc<LabelModel>>,
}

impl Global for LabelState {}

impl LabelState {
    pub fn init(cx: &mut App) {
        let this = LabelState { labels: vec![], active_label: None };
        cx.set_global(this);

        let conn = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |cx| {
            let db = conn.lock().await;
            let list = load_labels(db.clone()).await;
            let rc_list: Vec<Rc<LabelModel>> =
                list.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("all labels: {}", list.len());
            let _ = cx.update_global::<LabelState, _>(|state, _cx| {
                state.labels = rc_list;
            });
        })
        .detach();
    }

    pub fn set_labels(&mut self, labels: impl IntoIterator<Item = LabelModel>) {
        self.labels = labels.into_iter().map(Rc::new).collect();
        self.active_label = None;
    }
}
