use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::SectionModel;

use crate::todo_state::{DBState, ProjectState, SectionState};

// 刷新sections
async fn refresh_sections(cx: &mut AsyncApp, db: DatabaseConnection) {
    let sections = crate::service::load_sections(db).await;
    let rc_sections = sections.iter().map(|section| Rc::new(section.clone())).collect::<Vec<_>>();
    cx.update_global::<ProjectState, _>(|state, _| {
        state.sections = rc_sections.clone();
    });
    cx.update_global::<SectionState, _>(|state, _| {
        state.sections = rc_sections.clone();
    });
}
// 添加section
#[allow(unused)]
pub fn add_section(section: Rc<SectionModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if crate::service::add_section(section.clone(), db.clone()).await.is_ok() {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改section
#[allow(unused)]
pub fn update_section(section: Rc<SectionModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if crate::service::mod_section(section.clone(), db.clone()).await.is_ok() {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除section
#[allow(unused)]
pub fn delete_section(section: Rc<SectionModel>, cx: &mut App) {
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        if let Ok(_store) = crate::service::del_section(section.clone(), db.clone()).await {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
