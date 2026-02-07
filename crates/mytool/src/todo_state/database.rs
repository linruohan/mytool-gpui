use gpui::Global;
use sea_orm::DatabaseConnection;

pub struct DBState {
    pub conn: DatabaseConnection,
}
impl Global for DBState {}
pub async fn get_todo_conn() -> DatabaseConnection {
    todos::init_db().await.expect("init db failed")
}

impl DBState {
    // pub fn init(cx: &mut App) {
    //     let this = DBState { conn: Arc::new(Mutex::new(None)) };
    //     cx.set_global(this);
    //     // Load saved connections on startup
    //     cx.spawn(async move |cx| {
    //         if let Ok(conn) = todos::init_db().await {
    //             let _ = cx.update_global::<DBState, _>(|todo_state, _cx| {
    //                 todo_state.conn = Arc::new(Mutex::new(conn));
    //             });
    //         }
    //     })
    //       .detach();
    // }
}
