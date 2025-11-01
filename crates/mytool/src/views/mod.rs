mod boards;
mod item;
mod label;
mod project;

pub use boards::completed_board::CompletedBoard;
pub use boards::inbox_board::{InboxBoard, ItemClickEvent};
pub use boards::labels_board::LabelsBoard;
pub use boards::pin_board::PinBoard;
pub use boards::scheduled_board::ScheduledBoard;
pub use boards::today_board::TodayBoard;
use gpui::Global;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub use boards::*;
pub use item::*;
pub use label::*;
pub use project::*;

pub struct DBState {
    pub conn: Arc<Mutex<DatabaseConnection>>,
}
impl Global for DBState {}
pub async fn todo_database_init() -> Arc<Mutex<DatabaseConnection>> {
    let conn = todos::init_db().await.expect("init db failed");
    Arc::new(Mutex::new(conn))
}
