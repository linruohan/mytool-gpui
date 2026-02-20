use gpui::Global;
use sea_orm::DatabaseConnection;

/// 数据库连接状态
///
/// 存储全局数据库连接，供旧的状态管理代码使用。
/// 新代码建议使用 TodoStore，它会自动管理数据加载。
///
/// 注意：DatabaseConnection 内部已经使用了 Arc 进行连接池管理，
/// 所以克隆操作是轻量级的（只增加引用计数）。
pub struct DBState {
    pub conn: DatabaseConnection,
}

impl Global for DBState {}

/// 初始化数据库连接
///
/// 返回一个新的数据库连接实例。
pub async fn get_todo_conn() -> DatabaseConnection {
    todos::init_db().await.expect("init db failed")
}
