use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window, px, rgb,
};
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    popover::Popover,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use todos::enums::item_priority::ItemPriority;

use crate::create_button_wrapper;

#[derive(Clone)]
pub enum PriorityEvent {
    Selected(ItemPriority),
}

pub struct PriorityState {
    focus_handle: FocusHandle,
    priority: ItemPriority,
    popover_open: bool,
}

impl EventEmitter<PriorityEvent> for PriorityState {}

impl Focusable for PriorityState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn priority_label(priority: &ItemPriority) -> String {
    match priority {
        ItemPriority::HIGH => t!("todo.priority.high").to_string(),
        ItemPriority::MEDIUM => t!("todo.priority.medium").to_string(),
        ItemPriority::LOW => t!("todo.priority.low").to_string(),
        ItemPriority::NONE => t!("todo.priority.none").to_string(),
    }
}

fn priority_color(priority: &ItemPriority, muted: Hsla) -> Hsla {
    match priority {
        ItemPriority::NONE => muted,
        other => rgb(other.get_color()).into(),
    }
}

impl Render for PriorityState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let muted = cx.theme().muted_foreground;
        let trigger_color = priority_color(&self.priority, muted);
        let view = cx.entity();

        Popover::new("priority-popover")
            .p_0()
            .text_sm()
            .open(self.popover_open)
            .on_open_change(cx.listener(|this, open, _, cx| {
                this.popover_open = *open;
                cx.notify();
            }))
            .trigger(
                Button::new("item-priority")
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::FlagOutlineThickSymbolic)
                    .text_color(trigger_color)
                    .tooltip(priority_label(&self.priority)),
            )
            .child(v_flex().p_1().min_w(px(148.)).gap_1().children(
                ItemPriority::all().into_iter().map(move |priority| {
                    let color = priority_color(&priority, muted);
                    let view = view.clone();
                    Button::new(("priority-option", priority.clone() as i32 as usize))
                        .small()
                        .ghost()
                        .w_full()
                        .icon(IconName::FlagOutlineThickSymbolic)
                        .text_color(color)
                        .label(priority_label(&priority))
                        .on_click(move |_, _, cx| {
                            let priority = priority.clone();
                            view.update(cx, |this, cx| {
                                this.priority = priority.clone();
                                this.popover_open = false;
                                cx.emit(PriorityEvent::Selected(priority));
                                cx.notify();
                            });
                        })
                }),
            ))
    }
}

impl PriorityState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), priority: ItemPriority::NONE, popover_open: false }
    }

    pub fn priority(&self, _cx: &App) -> ItemPriority {
        self.priority.clone()
    }

    pub fn set_priority(
        &mut self,
        priority: impl Into<ItemPriority>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.priority = priority.into();
        cx.notify();
    }
}

create_button_wrapper!(PriorityButton, PriorityState, "item-priority");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_display_name() {
        assert_eq!("Priority 4: NONE", ItemPriority::NONE.display_name());
        assert_eq!("Priority 3: Low", ItemPriority::LOW.display_name());
        assert_eq!("Priority 2: Medium", ItemPriority::MEDIUM.display_name());
        assert_eq!("Priority 1: High", ItemPriority::HIGH.display_name());
    }

    #[test]
    fn test_priority_event_clone() {
        let event = PriorityEvent::Selected(ItemPriority::HIGH);
        let _cloned = event.clone();
    }
}
