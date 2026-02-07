//! Plugin system for extending application functionality
//!
//! This module provides a plugin system that allows for extending the application
//! with additional features and functionality.

mod config;
mod plugin;
mod registry;

pub use config::PluginConfig;
pub use plugin::{Plugin, PluginMetadata};
pub use registry::PluginRegistry;

/// Initialize the plugin system
pub fn init_plugins(cx: &mut gpui::App) {
    PluginRegistry::init(cx);
}
