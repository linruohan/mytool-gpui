use serde::Deserialize;

/// 数据库配置结构体
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseConfig {
    host: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    schema: Option<String>,
    #[serde(default = "default_pool_size")]
    pool_size: Option<u32>,
}

/// 默认数据库连接池大小
fn default_pool_size() -> Option<u32> {
    Some(10)
}

impl DatabaseConfig {
    /// 获取数据库主机
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("127.0.0.1")
    }

    /// 获取数据库端口
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(5432)
    }

    /// 获取数据库用户
    pub fn user(&self) -> &str {
        self.user.as_deref().unwrap_or("postgres")
    }

    /// 获取数据库密码
    pub fn password(&self) -> &str {
        self.password.as_deref().unwrap_or("postgres")
    }

    /// 获取数据库名称
    pub fn database(&self) -> &str {
        self.database.as_deref().unwrap_or("postgres")
    }

    /// 获取数据库 schema
    pub fn schema(&self) -> &str {
        self.schema.as_deref().unwrap_or("public")
    }

    /// 获取数据库连接池大小
    pub fn pool_size(&self) -> u32 {
        self.pool_size.unwrap_or(10)
    }
}
