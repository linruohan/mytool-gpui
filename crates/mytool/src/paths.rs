
use std::{env, path::PathBuf, sync::OnceLock};

pub struct Paths {
    pub path_env: String,
    pub cache: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
}

pub static NAME: &str = "MyTools";

impl Paths {
    pub fn new() -> Self {
        let username = whoami::username();
        #[cfg(target_os = "macos")]
        let user_dir = PathBuf::from("/Users").join(username.clone());
        #[cfg(target_os = "linux")]
        let user_dir = PathBuf::from("/home").join(username.clone());
        #[cfg(target_os = "windows")]
        let user_dir = PathBuf::from("C:\\Users").join(username.clone());
        Self {
            #[cfg(target_os = "macos")]
            path_env: format!(
                "/opt/homebrew/bin:/usr/local/bin:/Users/{}/.nix-profile/bin",
                username
            ),
            #[cfg(target_os = "linux")]
            path_env: format!(
                "/opt/homebrew/bin:/usr/local/bin:/home/{}/.nix-profile/bin",
                username
            ),
            #[cfg(target_os = "windows")]
            path_env: format!(
                "C:\\Program Files\\Common Files;C:\\Program Files;C:\\Users\\{}\\AppData\\Local\\bin;C:\\Chocolatey\\bin",
                username
            ),
            #[cfg(target_os = "macos")]
            cache: user_dir.clone().join("Library/Caches").join(NAME),
            #[cfg(target_os = "linux")]
            cache: user_dir.clone().join(".cache").join(NAME),
            config: user_dir.clone().join(".config").join(NAME),
            #[cfg(target_os = "macos")]
            data: user_dir
                .clone()
                .join("Library/Application Support")
                .join(NAME),
            #[cfg(target_os = "linux")]
            data: user_dir.clone().join(".local/share").join(NAME),
            cache: Default::default(),
            data: Default::default(),
        }
    }
}

pub fn paths() -> &'static Paths {
    static PATHS: OnceLock<Paths> = OnceLock::new();
    PATHS.get_or_init(Paths::new)
}
