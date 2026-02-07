// Button component macros have been consolidated into dropdown_button.rs
//
// The `create_button_wrapper!` macro is now defined in dropdown_button.rs
// and provides a unified way to create button wrapper components.
//
// Usage:
// ```ignore
// create_button_wrapper!(ButtonName, StateName, "button-id");
// ```
//
// This macro generates:
// - Sizable trait implementation
// - Focusable trait implementation
// - Styled trait implementation
// - new() constructor
// - RenderOnce trait implementation
//
// All button wrappers now use this single macro for consistency.
