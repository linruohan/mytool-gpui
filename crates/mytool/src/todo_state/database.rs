use gpui::Global;
use sea_orm::DatabaseConnection;

/// 数据库连接状态
///
/// 存储全局数据库连接，供旧的状态管理代码使用。
/// 新代码建议使用 TodoStore，它会自动管理数据加载。
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

impl DBState {
    /// 创建一个占位的 DBState（需要在后续异步设置真正的连接）
    ///
    /// 注意：这只是一个临时方案，用于支持旧代码的渐进式迁移。
    /// 新代码应该使用 TodoStore。
    pub fn placeholder() -> Self {
        // 使用内存数据库作为占位符
        // 实际连接会在 state_init 中异步设置
        panic!("DBState placeholder should not be used directly. Use TodoStore instead.")
    }
}
