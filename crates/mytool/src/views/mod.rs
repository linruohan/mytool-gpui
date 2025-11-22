mod boards;
mod item;
mod label;
mod priority_list;
mod project;

use std::sync::Arc;

pub use boards::{
    completed_board::CompletedBoard,
    inbox_board::{InboxBoard, ItemClickEvent},
    labels_board::LabelsBoard,
    pin_board::PinBoard,
    scheduled_board::ScheduledBoard,
    today_board::TodayBoard,
    *,
};
use gpui::Global;
pub use item::*;
pub use label::*;
pub use project::*;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

pub struct DBState {
    pub conn: Arc<Mutex<DatabaseConnection>>,
}
impl Global for DBState {}
pub async fn todo_database_init() -> Arc<Mutex<DatabaseConnection>> {
    let conn = todos::init_db().await.expect("init db failed");
    Arc::new(Mutex::new(conn))
}
