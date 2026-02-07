//! Plugin trait and metadata definitions
//!
//! This module defines the core Plugin trait that all plugins must implement,
//! as well as metadata structures for plugin information.

use gpui::{App, Window};
use serde::{Deserialize, Serialize};

/// Plugin metadata containing information about a plugin
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier for the plugin
    pub id: String,
    /// Display name of the plugin
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Short description of the plugin
    pub description: String,
    /// Author of the plugin
    pub author: String,
    /// Whether the plugin is enabled by default
    pub enabled_by_default: bool,
}

/// Core plugin trait that all plugins must implement
pub trait Plugin {
    /// Get metadata about the plugin
    fn metadata(&self) -> PluginMetadata;

    /// Initialize the plugin
    fn init(&mut self, window: &mut Window, cx: &mut App);

    /// Clean up plugin resources
    fn cleanup(&mut self, cx: &mut App);

    /// Check if the plugin is enabled
    fn is_enabled(&self) -> bool;

    /// Enable or disable the plugin
    fn set_enabled(&mut self, enabled: bool);
}
