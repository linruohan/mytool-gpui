//! Todo 偏好：可执行文件旁的 `todo_prefs.json`

use std::path::PathBuf;

use gpui::Global;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoPrefs {
    #[serde(default = "default_true")]
    pub reminders_enabled: bool,
    #[serde(default = "default_true")]
    pub confirm_on_delete: bool,
    #[serde(default = "default_true")]
    pub complete_sound: bool,
    /// 启动时打开的看板：0 收件箱 … 5 已完成
    #[serde(default)]
    pub startup_board: u8,
    /// 界面缩放，相对 16px rem（约 0.8–1.6）
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
}

fn default_true() -> bool {
    true
}

fn default_ui_scale() -> f32 {
    1.0
}

impl Default for TodoPrefs {
    fn default() -> Self {
        Self {
            reminders_enabled: true,
            confirm_on_delete: true,
            complete_sound: true,
            startup_board: 0,
            ui_scale: default_ui_scale(),
        }
    }
}

impl Global for TodoPrefs {}

impl TodoPrefs {
    pub fn load() -> Self {
        let path = prefs_path();
        let Ok(bytes) = std::fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = prefs_path();
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, bytes);
        }
    }
}

fn prefs_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("todo_prefs.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prefs_enable_reminders_and_confirm() {
        let prefs = TodoPrefs::default();
        assert!(prefs.reminders_enabled);
        assert!(prefs.confirm_on_delete);
        assert!(prefs.complete_sound);
        assert_eq!(prefs.startup_board, 0);
        assert_eq!(prefs.ui_scale, 1.0);
    }

    #[test]
    fn missing_ui_scale_in_json_defaults_to_one() {
        let prefs: TodoPrefs = serde_json::from_slice(
            br#"{"reminders_enabled":true,"confirm_on_delete":true,"complete_sound":true,"startup_board":1}"#,
        )
        .unwrap();
        assert_eq!(prefs.ui_scale, 1.0);
        assert_eq!(prefs.startup_board, 1);
    }
}
