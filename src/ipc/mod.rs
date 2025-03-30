pub mod client;
pub mod server;
#[cfg(target_os = "windows")]
pub const SOCKET_PATH: &str = r"\\.\pipe\linruohan_pipe";
#[cfg(not(target_os = "windows"))]
pub const SOCKET_PATH: &str = "/tmp/linruohan.sock";
