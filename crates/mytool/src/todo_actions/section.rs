use std::rc::Rc;

use gpui::{App, AsyncApp};
use sea_orm::DatabaseConnection;
use todos::entity::SectionModel;

use crate::todo_state::{DBState, SectionState};

// 刷新sections
#[allow(unused)]
async fn refresh_sections(cx: &mut AsyncApp, db: DatabaseConnection) {
    let sections = crate::service::load_sections(db).await;
    cx.update_global::<SectionState, _>(|state, _| {
        state.set_sections(sections);
    })
    .ok();
}
// 添加section
#[allow(unused)]
pub fn add_section(section: Rc<SectionModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::add_section(section.clone(), db.clone()).await.is_ok() {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
// 修改section
#[allow(unused)]
pub fn update_section(section: Rc<SectionModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if crate::service::mod_section(section.clone(), db.clone()).await.is_ok() {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
// 删除section
#[allow(unused)]
pub fn delete_section(section: Rc<SectionModel>, cx: &mut App) {
    let conn = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        let db = conn.lock().await;
        if let Ok(_store) = crate::service::del_section(section.clone(), db.clone()).await {
            refresh_sections(cx, db.clone()).await;
        }
    })
    .detach();
}
