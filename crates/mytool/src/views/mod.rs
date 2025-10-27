mod board;
mod board_container;
mod completed_board;
mod inbox_board;
mod item;
mod label;
mod labels_board;
mod pin_board;
mod project;
mod scheduled_board;
mod today_board;

pub use completed_board::CompletedBoard;
use gpui::Global;
pub use inbox_board::{InboxBoard, ItemClickEvent};
pub use labels_board::LabelsBoard;
pub use pin_board::PinBoard;
pub use scheduled_board::ScheduledBoard;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
pub use today_board::TodayBoard;
use tokio::sync::Mutex;

pub use board::BoardType;
pub use board_container::{Board, BoardContainer};
pub use item::{ItemListDelegate, ItemListItem};
pub use label::{LabelListDelegate, LabelListItem};
pub use project::{ProjectListDelegate, ProjectListItem};

pub struct DBState {
    pub conn: Arc<Mutex<DatabaseConnection>>,
}
impl Global for DBState {}
pub async fn todo_database_init() -> Arc<Mutex<DatabaseConnection>> {
    let conn = todos::init_db().await.expect("init db failed");
    Arc::new(Mutex::new(conn))
}
