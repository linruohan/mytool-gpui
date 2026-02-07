//! Plugin configuration and feature toggles
//!
//! This module provides configuration management for plugins and feature toggles
//! that allow for enabling or disabling specific features.

use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

/// Plugin configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Enabled plugins
    enabled_plugins: Vec<String>,
    /// Feature toggles
    feature_toggles: HashMap<String, bool>,
}

impl PluginConfig {
    /// Load plugin configuration from file
    pub fn load() -> Self {
        let config_path = Path::new("plugins.toml");

        if config_path.exists() {
            match File::open(config_path) {
                Ok(mut file) => {
                    let mut content = String::new();
                    if file.read_to_string(&mut content).is_ok()
                        && let Ok(config) = toml::from_str(&content)
                    {
                        return config;
                    }
                },
                Err(_) => {
                    // File exists but can't be opened, use default
                },
            }
        }

        // Return default configuration
        Self::default()
    }

    /// Save plugin configuration to file
    pub fn save(&self) {
        let config_path = Path::new("plugins.toml");

        if let Ok(content) = toml::to_string_pretty(self)
            && let Ok(mut file) = File::create(config_path)
        {
            let _ = file.write_all(content.as_bytes());
        }
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.feature_toggles.get(feature).copied().unwrap_or(false)
    }

    /// Enable or disable a feature
    pub fn set_feature_enabled(&mut self, feature: &str, enabled: bool) {
        self.feature_toggles.insert(feature.to_string(), enabled);
        self.save();
    }

    /// Get all feature toggles
    pub fn feature_toggles(&self) -> &HashMap<String, bool> {
        &self.feature_toggles
    }

    /// Get enabled plugins
    pub fn enabled_plugins(&self) -> &Vec<String> {
        &self.enabled_plugins
    }

    /// Set enabled plugins
    pub fn set_enabled_plugins(&mut self, plugins: Vec<String>) {
        self.enabled_plugins = plugins;
        self.save();
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled_plugins: vec![],
            feature_toggles: HashMap::from([
                ("advanced_search".to_string(), false),
                ("dark_mode".to_string(), true),
                ("calendar_integration".to_string(), false),
                ("reminders".to_string(), true),
            ]),
        }
    }
}
