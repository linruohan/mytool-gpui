use gpui::{
    App, Context, Corner, EventEmitter, FocusHandle, Focusable, InteractiveElement, ParentElement,
    Render, SharedString, Window, px,
};
use gpui_component::{IconName, button::Button, menu::DropdownMenu, v_flex};
use todos::enums::item_priority::ItemPriority;

use crate::{
    components::dropdown_button::{DropdownEvent, DropdownState},
    create_button_wrapper,
};

pub enum PriorityEvent {
    Selected(i32),
}

pub struct PriorityState {
    inner: DropdownState<ItemPriority>,
}

impl EventEmitter<PriorityEvent> for PriorityState {}

impl Focusable for PriorityState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.inner.focus_handle(cx)
    }
}

impl PriorityState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { inner: DropdownState::new(_window, cx) }
    }

    pub fn priority(&self) -> ItemPriority {
        self.inner.selected().unwrap_or(ItemPriority::NONE)
    }

    pub fn set_priority(
        &mut self,
        priority: impl Into<ItemPriority>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.inner.set_selected(Some(priority.into()), _window, cx);
    }

    fn on_action_select(
        &mut self,
        action: &DropdownEvent<ItemPriority>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let DropdownEvent::Selected(priority) = action {
            self.inner.selected = Some(priority.clone());
            cx.emit(PriorityEvent::Selected(priority.clone() as i32));
            cx.notify();
        }
    }
}

impl Render for PriorityState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let selected = self.inner.selected.clone();
        v_flex().on_action(_cx.listener(Self::on_action_select)).child(
            Button::new("priority")
                .outline()
                .tooltip("set priority")
                .icon(IconName::FlagOutlineThickSymbolic)
                .label(SharedString::from(selected.unwrap_or(ItemPriority::NONE).display_name()))
                .dropdown_menu_with_anchor(
                    Corner::TopLeft,
                    move |this: gpui_component::menu::PopupMenu, _, _| {
                        let mut this = this.scrollable(true).max_h(px(400.));
                        for priority in ItemPriority::all() {
                            this = this.menu(
                                SharedString::from(priority.display_name()),
                                Box::new(DropdownEvent::Selected(priority)),
                            );
                        }
                        this.min_w(px(100.))
                    },
                ),
        )
    }
}

create_button_wrapper!(PriorityButton, PriorityState, "item-priority");
