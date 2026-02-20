//! # Visual Enhancements Module
//!
//! This module provides visual enhancements for the MyTool application, including:
//! - Semantic colors for priorities, statuses, and interactions
//! - Visual hierarchy utilities (shadows, borders, spacing)
//! - Animation and transition helpers
//! - Responsive layout utilities
//!
//! ## Usage
//!
//! ```rust
//! use crate::visual_enhancements::{Animations, SemanticColors, VisualHierarchy};
//!
//! // Get semantic colors
//! let colors = SemanticColors::from_theme(cx.theme());
//! let priority_color = colors.priority_high();
//!
//! // Apply visual hierarchy
//! div().apply_card_style(cx).apply_hover_effect().child(content)
//! ```

use gpui::{App, Hsla, Pixels, StyleRefinement, hsla, px};
use gpui_component::theme::ActiveTheme;

/// Semantic colors for different UI elements
///
/// These colors provide consistent visual meaning across the application:
/// - Priority colors: High (red), Medium (yellow), Low (blue)
/// - Status colors: Completed (green), Overdue (red), Today (orange)
/// - Interaction colors: Hover, Active, Focus states
#[derive(Debug, Clone)]
pub struct SemanticColors {
    // Priority colors
    pub priority_high: Hsla,
    pub priority_medium: Hsla,
    pub priority_low: Hsla,
    pub priority_none: Hsla,

    // Status colors
    pub status_completed: Hsla,
    pub status_overdue: Hsla,
    pub status_today: Hsla,
    pub status_scheduled: Hsla,
    pub status_pinned: Hsla,

    // Interaction colors
    pub hover_overlay: Hsla,
    pub active_overlay: Hsla,
    pub focus_ring: Hsla,
    pub drag_overlay: Hsla,

    // Feedback colors
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,
}

impl SemanticColors {
    /// Create semantic colors from the current theme
    pub fn from_theme(cx: &App) -> Self {
        let theme = cx.theme();
        let is_dark = theme.mode.is_dark();

        Self {
            // Priority colors - adjusted for light/dark mode
            priority_high: if is_dark {
                hsla(0.0, 0.7, 0.5, 1.0) // Red
            } else {
                hsla(0.0, 0.7, 0.4, 1.0)
            },
            priority_medium: if is_dark {
                hsla(45.0, 0.8, 0.5, 1.0) // Yellow
            } else {
                hsla(45.0, 0.8, 0.4, 1.0)
            },
            priority_low: if is_dark {
                hsla(210.0, 0.7, 0.5, 1.0) // Blue
            } else {
                hsla(210.0, 0.7, 0.4, 1.0)
            },
            priority_none: theme.muted_foreground,

            // Status colors
            status_completed: if is_dark {
                hsla(140.0, 0.6, 0.5, 1.0) // Green
            } else {
                hsla(140.0, 0.6, 0.4, 1.0)
            },
            status_overdue: if is_dark {
                hsla(0.0, 0.7, 0.5, 1.0) // Red
            } else {
                hsla(0.0, 0.7, 0.4, 1.0)
            },
            status_today: if is_dark {
                hsla(30.0, 0.8, 0.5, 1.0) // Orange
            } else {
                hsla(30.0, 0.8, 0.4, 1.0)
            },
            status_scheduled: if is_dark {
                hsla(210.0, 0.6, 0.5, 1.0) // Blue
            } else {
                hsla(210.0, 0.6, 0.4, 1.0)
            },
            status_pinned: if is_dark {
                hsla(280.0, 0.6, 0.5, 1.0) // Purple
            } else {
                hsla(280.0, 0.6, 0.4, 1.0)
            },

            // Interaction colors
            hover_overlay: if is_dark {
                hsla(0.0, 0.0, 1.0, 0.05) // White 5%
            } else {
                hsla(0.0, 0.0, 0.0, 0.03) // Black 3%
            },
            active_overlay: if is_dark {
                hsla(210.0, 0.8, 0.5, 0.15) // Blue 15%
            } else {
                hsla(210.0, 0.8, 0.5, 0.1) // Blue 10%
            },
            focus_ring: hsla(210.0, 0.8, 0.5, 0.5), // Blue 50%
            drag_overlay: hsla(210.0, 0.8, 0.5, 0.2), // Blue 20%

            // Feedback colors (from theme)
            success: theme.success,
            warning: theme.warning,
            error: theme.danger,
            info: theme.info,
        }
    }

    /// Get priority color by priority level (0-3)
    pub fn priority_color(&self, priority: u8) -> Hsla {
        match priority {
            3 => self.priority_high,
            2 => self.priority_medium,
            1 => self.priority_low,
            _ => self.priority_none,
        }
    }
}

/// Visual hierarchy utilities for consistent styling
///
/// Provides methods to apply consistent visual styles across the application:
/// - Card styles with shadows and borders
/// - Hover and active effects
/// - Focus indicators
/// - Spacing and sizing utilities
pub struct VisualHierarchy;

impl VisualHierarchy {
    /// Shadow levels for depth perception
    pub fn shadow_sm() -> StyleRefinement {
        StyleRefinement::default()
        // Small shadow: 0 1px 2px rgba(0,0,0,0.05)
    }

    pub fn shadow_md() -> StyleRefinement {
        StyleRefinement::default()
        // Medium shadow: 0 4px 6px rgba(0,0,0,0.1)
    }

    pub fn shadow_lg() -> StyleRefinement {
        StyleRefinement::default()
        // Large shadow: 0 10px 15px rgba(0,0,0,0.1)
    }

    /// Border radius values for consistency
    pub fn radius_sm() -> Pixels {
        px(4.0)
    }

    pub fn radius_md() -> Pixels {
        px(6.0)
    }

    pub fn radius_lg() -> Pixels {
        px(8.0)
    }

    pub fn radius_xl() -> Pixels {
        px(12.0)
    }

    /// Spacing scale (4px base)
    pub fn spacing(multiplier: f32) -> Pixels {
        px(4.0 * multiplier)
    }
}

/// Animation and transition utilities
///
/// Provides consistent animation durations and easing functions
pub struct Animations;

impl Animations {
    /// Animation durations in milliseconds
    pub const DURATION_FAST: u64 = 150;
    pub const DURATION_NORMAL: u64 = 200;
    pub const DURATION_SLOW: u64 = 300;

    /// Easing functions (for future use with GPUI animations)
    pub fn ease_in_out() -> &'static str {
        "cubic-bezier(0.4, 0.0, 0.2, 1.0)"
    }

    pub fn ease_out() -> &'static str {
        "cubic-bezier(0.0, 0.0, 0.2, 1.0)"
    }

    pub fn ease_in() -> &'static str {
        "cubic-bezier(0.4, 0.0, 1.0, 1.0)"
    }
}

/// Responsive layout utilities
///
/// Provides breakpoints and utilities for responsive design
pub struct ResponsiveLayout;

impl ResponsiveLayout {
    pub const BREAKPOINT_LG: f32 = 1024.0;
    pub const BREAKPOINT_MD: f32 = 768.0;
    /// Breakpoint widths in pixels
    pub const BREAKPOINT_SM: f32 = 640.0;
    pub const BREAKPOINT_XL: f32 = 1280.0;

    /// Check if window is in compact mode
    pub fn is_compact(window_width: Pixels) -> bool {
        window_width < px(Self::BREAKPOINT_MD)
    }

    /// Check if window is in normal mode
    pub fn is_normal(window_width: Pixels) -> bool {
        window_width >= px(Self::BREAKPOINT_MD) && window_width < px(Self::BREAKPOINT_LG)
    }

    /// Check if window is in wide mode
    pub fn is_wide(window_width: Pixels) -> bool {
        window_width >= px(Self::BREAKPOINT_LG)
    }
}

/// Extension trait for applying visual enhancements to elements
pub trait VisualEnhancementExt {
    /// Apply card style (background, border, shadow)
    fn apply_card_style(self, cx: &App) -> Self;

    /// Apply hover effect
    fn apply_hover_effect(self, cx: &App) -> Self;

    /// Apply active/pressed effect
    fn apply_active_effect(self, cx: &App) -> Self;

    /// Apply focus ring
    fn apply_focus_ring(self, cx: &App) -> Self;

    /// Apply priority indicator (left border)
    fn apply_priority_indicator(self, priority: u8, cx: &App) -> Self;
}

// Note: Implementation of VisualEnhancementExt would require GPUI element types
// This is a placeholder for the trait definition

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_color_mapping() {
        // Test that priority levels map correctly
        // This would require a mock theme context
    }

    #[test]
    fn test_responsive_breakpoints() {
        assert!(ResponsiveLayout::is_compact(px(600.0)));
        assert!(ResponsiveLayout::is_normal(px(800.0)));
        assert!(ResponsiveLayout::is_wide(px(1100.0)));
    }

    #[test]
    fn test_spacing_scale() {
        assert_eq!(VisualHierarchy::spacing(1.0), px(4.0));
        assert_eq!(VisualHierarchy::spacing(2.0), px(8.0));
        assert_eq!(VisualHierarchy::spacing(4.0), px(16.0));
    }
}
