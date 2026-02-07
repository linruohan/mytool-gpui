use serde::Deserialize;

/// 服务器配置结构体
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ServerConfig {
    port: Option<u16>,
    host: Option<String>,
}

impl ServerConfig {
    /// 获取服务器端口
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(8080)
    }

    /// 获取服务器主机
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("0.0.0.0")
    }
}
