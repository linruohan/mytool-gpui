use serde::Deserialize;

/// 数据库配置结构体
///
/// 支持 SQLite 和 PostgreSQL 等数据库类型
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseConfig {
    /// 数据库类型: sqlite, postgresql 等
    #[serde(default = "default_db_type")]
    db_type: Option<String>,
    /// SQLite 文件路径（仅当 db_type=sqlite 时使用）
    #[serde(default)]
    path: Option<String>,
    /// 数据库主机（仅用于网络数据库）
    host: Option<String>,
    /// 数据库端口（仅用于网络数据库）
    port: Option<u16>,
    /// 数据库用户（仅用于网络数据库）
    user: Option<String>,
    /// 数据库密码（仅用于网络数据库）
    password: Option<String>,
    /// 数据库名称
    database: Option<String>,
    /// 数据库 schema（仅 PostgreSQL）
    schema: Option<String>,
    /// 数据库连接池大小
    #[serde(default = "default_pool_size")]
    pool_size: Option<u32>,
}

/// 默认数据库类型
fn default_db_type() -> Option<String> {
    Some("sqlite".to_string())
}

/// 默认数据库连接池大小
fn default_pool_size() -> Option<u32> {
    Some(10)
}

impl DatabaseConfig {
    /// 检查是否为 SQLite 数据库
    pub fn is_sqlite(&self) -> bool {
        self.db_type.as_deref().map(|t| t == "sqlite").unwrap_or(true)
    }

    /// 获取数据库类型
    pub fn db_type(&self) -> &str {
        self.db_type.as_deref().unwrap_or("sqlite")
    }

    /// 获取 SQLite 文件路径
    ///
    /// # Panics
    /// 如果不是 SQLite 类型数据库但尝试获取路径会 panic
    pub fn sqlite_path(&self) -> &str {
        self.path.as_deref().unwrap_or("db.sqlite")
    }

    /// 获取完整的 SQLite 连接 URL
    pub fn sqlite_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.sqlite_path())
    }

    /// 获取数据库主机
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// 获取数据库端口
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// 获取数据库用户
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// 获取数据库密码
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// 获取数据库名称
    pub fn database(&self) -> &str {
        self.database.as_deref().unwrap_or("mytool")
    }

    /// 获取数据库 schema
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// 获取数据库连接池大小
    pub fn pool_size(&self) -> u32 {
        self.pool_size.unwrap_or(10)
    }
}
