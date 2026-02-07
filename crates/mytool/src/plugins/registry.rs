//! Plugin registry for managing plugins
//!
//! This module provides a registry for managing plugins, including loading,
//! initializing, and accessing plugins.

use std::collections::HashMap;

use gpui::{App, Global};

use super::{Plugin, PluginConfig};

/// Plugin registry for managing all plugins
#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin + Send + Sync>>,
    config: PluginConfig,
}

impl Global for PluginRegistry {}

impl PluginRegistry {
    /// Initialize the plugin registry
    pub fn init(cx: &mut App) {
        let config = PluginConfig::load();

        cx.set_global(Self { plugins: HashMap::new(), config });
    }

    /// Get the plugin registry (read-only)
    pub fn get(cx: &mut App) -> &Self {
        cx.global::<Self>()
    }

    /// Get the plugin registry (mutable)
    pub fn get_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Get all registered plugins
    pub fn plugins(&self) -> &HashMap<String, Box<dyn Plugin + Send + Sync>> {
        &self.plugins
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, id: &str) -> Option<&Box<dyn Plugin + Send + Sync>> {
        self.plugins.get(id)
    }

    /// Check if a plugin is enabled
    pub fn is_plugin_enabled(&self, id: &str) -> bool {
        self.plugins.get(id).map(|plugin| plugin.is_enabled()).unwrap_or(false)
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.config.is_feature_enabled(feature)
    }

    /// Get all feature toggles
    pub fn feature_toggles(&self) -> &HashMap<String, bool> {
        self.config.feature_toggles()
    }

    /// Register a new plugin
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin + Send + Sync>) {
        let id = plugin.metadata().id;
        self.plugins.insert(id, plugin);
    }

    /// Enable or disable a plugin
    pub fn set_plugin_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.set_enabled(enabled);
        }
    }

    /// Initialize all enabled plugins
    pub fn init_plugins(&mut self, window: &mut gpui::Window, cx: &mut App) {
        for plugin in self.plugins.values_mut() {
            if plugin.is_enabled() {
                plugin.init(window, cx);
            }
        }
    }

    /// Clean up all plugins
    pub fn cleanup_plugins(&mut self, cx: &mut App) {
        for plugin in self.plugins.values_mut() {
            plugin.cleanup(cx);
        }
    }

    /// Enable or disable a feature
    pub fn set_feature_enabled(&mut self, feature: &str, enabled: bool) {
        self.config.set_feature_enabled(feature, enabled);
    }

    /// Get mutable access to the plugin config
    pub fn config(&mut self) -> &mut PluginConfig {
        &mut self.config
    }
}
