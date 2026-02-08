use gpui::{Context, EventEmitter, Focusable, ParentElement, Render, Window};
use gpui_component::IconName;
use todos::enums::item_priority::ItemPriority;

use crate::{
    components::drop_btn::{
        render_dropdown_button, DropdownButtonStateTrait, DropdownEvent, DropdownState,
    },
    create_button_wrapper,
};

#[derive(Clone)]
pub enum PriorityEvent {
    Selected(ItemPriority),
}

pub struct PriorityState {
    inner: DropdownState<ItemPriority>,
}

impl EventEmitter<PriorityEvent> for PriorityState {}

impl Focusable for PriorityState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl Render for PriorityState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        render_dropdown_button::<ItemPriority, Self>(self, window, cx)
    }
}

impl DropdownButtonStateTrait<ItemPriority> for PriorityState {
    type EventType = PriorityEvent;

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx) }
    }

    fn inner(&self) -> &DropdownState<ItemPriority> {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut DropdownState<ItemPriority> {
        &mut self.inner
    }

    fn on_action_select(
        &mut self,
        action: &DropdownEvent<ItemPriority>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DropdownEvent::Selected(priority) = action;
        self.inner.selected = Some(priority.clone());
        cx.emit(PriorityEvent::Selected(priority.clone()));
        cx.notify();
    }

    fn button_id(&self) -> &'static str {
        "priority"
    }

    fn tooltip_text(&self) -> &'static str {
        "set priority"
    }

    fn selected_display_name(&self, _cx: &mut Context<Self>) -> String {
        self.priority().display_name().to_string()
    }

    fn menu_options(&self, _cx: &mut Context<Self>) -> Vec<(String, ItemPriority)> {
        ItemPriority::all().into_iter().map(|p| (p.display_name().to_string(), p)).collect()
    }

    fn button_icon(&self) -> Option<IconName> {
        Some(IconName::FlagOutlineThickSymbolic)
    }

    fn min_width(&self) -> f32 {
        100.0
    }
}

impl PriorityState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(window, cx) }
    }

    pub fn priority(&self) -> ItemPriority {
        self.selected().unwrap_or(ItemPriority::NONE)
    }

    pub fn set_priority(
        &mut self,
        priority: impl Into<ItemPriority>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected(Some(priority.into()), window, cx);
    }
}

create_button_wrapper!(PriorityButton, PriorityState, "item-priority");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_state_new() {
        // Test that PriorityState can be created
        // Note: Full initialization requires a Window and Context, which is complex in tests
        // This test ensures the type is properly defined
    }

    #[test]
    fn test_priority_display_name() {
        assert_eq!("None", ItemPriority::NONE.display_name());
        assert_eq!("Low", ItemPriority::LOW.display_name());
        assert_eq!("Medium", ItemPriority::MEDIUM.display_name());
        assert_eq!("High", ItemPriority::HIGH.display_name());
    }

    #[test]
    fn test_priority_event_clone() {
        let event = PriorityEvent::Selected(ItemPriority::HIGH);
        // Test that event can be cloned (simple clone)
        let _cloned = event.clone();
    }
}
