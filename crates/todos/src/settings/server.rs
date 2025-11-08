use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ServerConfig {
    port: Option<u16>,
}
impl ServerConfig {
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(8080)
    }
}
