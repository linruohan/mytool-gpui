use std::sync::Arc;

use gpui::Global;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;
pub struct DBState {
    pub conn: Arc<Mutex<DatabaseConnection>>,
}
impl Global for DBState {}
pub async fn get_todo_conn() -> Arc<Mutex<DatabaseConnection>> {
    let conn = todos::init_db().await.expect("init db failed");
    Arc::new(Mutex::new(conn))
}

impl DBState {
    // pub fn init(cx: &mut App) {
    //     let this = DBState { conn: Arc::new(Mutex::new(None)) };
    //     cx.set_global(this);
    //     // Load saved connections on startup
    //     cx.spawn(async move |cx| {
    //         if let Ok(conn) = todos::init_db().await {
    //             let _ = cx.update_global::<DBState, _>(|state, _cx| {
    //                 state.conn = Arc::new(Mutex::new(conn));
    //             });
    //         }
    //     })
    //       .detach();
    // }
}
