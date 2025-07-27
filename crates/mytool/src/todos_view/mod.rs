mod board;
mod completed_board;
mod inbox_board;
mod labels_board;
mod pin_board;
mod project_item;
mod scheduled_board;
mod today_board;
mod todo_container;

pub use completed_board::CompletedBoard;
use gpui::Global;
pub use inbox_board::InboxBoard;
pub use labels_board::LabelsBoard;
pub use pin_board::PinBoard;
pub use scheduled_board::ScheduledBoard;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
pub use today_board::TodayBoard;
use tokio::sync::Mutex;

pub use board::{Board, BoardType};
pub use project_item::ProjectItem;
pub use todo_container::TodoContainer;

pub struct DBState {
    pub conn: Arc<Mutex<DatabaseConnection>>,
}
impl Global for DBState {}
pub async fn todo_database_init() -> Arc<Mutex<DatabaseConnection>> {
    let conn = todos::init_db().await.expect("init db failed");
    Arc::new(Mutex::new(conn))
}