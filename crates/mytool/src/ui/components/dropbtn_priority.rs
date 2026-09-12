use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, px, rgb,
};
use gpui_component::{
    Icon, IndexPath, Sizable, h_flex,
    searchable_list::SearchableListItem,
    select::{Select, SelectEvent, SelectState},
};
use gpui_kit::assets::IconName;
use todos::enums::item_priority::ItemPriority;

use crate::create_button_wrapper;

#[derive(Clone)]
pub enum PriorityEvent {
    Selected(ItemPriority),
}

#[derive(Clone, PartialEq, Eq)]
struct PriorityOption(ItemPriority);

impl SearchableListItem for PriorityOption {
    type Value = ItemPriority;

    fn title(&self) -> SharedString {
        self.0.display_name().into()
    }

    fn value(&self) -> &Self::Value {
        &self.0
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(
            Icon::new(IconName::FlagOutlineThickSymbolic)
                .text_color(rgb(self.0.get_color()))
                .into_any_element(),
        )
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .gap_1()
            .items_center()
            .child(
                Icon::new(IconName::FlagOutlineThickSymbolic).text_color(rgb(self.0.get_color())),
            )
            .child(self.0.display_name())
    }
}

pub struct PriorityState {
    select: Entity<SelectState<Vec<PriorityOption>>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PriorityEvent> for PriorityState {}

impl Focusable for PriorityState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.select.focus_handle(cx)
    }
}

impl Render for PriorityState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Select::new(&self.select).small().appearance(true).placeholder("Priority").w(px(88.))
    }
}

impl PriorityState {
    fn all_options() -> Vec<PriorityOption> {
        ItemPriority::all().into_iter().map(PriorityOption).collect()
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let none_index =
            ItemPriority::all().iter().position(|p| *p == ItemPriority::NONE).unwrap_or(3);

        let select = cx.new(|cx| {
            SelectState::new(
                Self::all_options(),
                Some(IndexPath::default().row(none_index)),
                window,
                cx,
            )
        });

        let _subscriptions =
            vec![cx.subscribe(&select, |_, _, event: &SelectEvent<Vec<PriorityOption>>, cx| {
                if let SelectEvent::Confirm(Some(priority)) = event {
                    cx.emit(PriorityEvent::Selected(priority.clone()));
                }
            })];

        Self { select, _subscriptions }
    }

    pub fn priority(&self, cx: &App) -> ItemPriority {
        self.select.read(cx).selected_value().cloned().unwrap_or(ItemPriority::NONE)
    }

    pub fn set_priority(
        &mut self,
        priority: impl Into<ItemPriority>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let priority = priority.into();
        self.select.update(cx, |select, cx| {
            select.set_selected_value(&priority, window, cx);
        });
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
