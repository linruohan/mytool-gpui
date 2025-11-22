use gpui::{
    Action, App, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, StyleSized, StyledExt as _, button::Button,
    menu::DropdownMenu, v_flex,
};
use serde::Deserialize;
use todos::enums::item_priority::ItemPriority;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = priority, no_json)]
struct PriorityInfo(i32);

pub enum PriorityEvent {
    Selected(i32),
}
pub struct PriorityState {
    focus_handle: FocusHandle,
    pub priority: ItemPriority,
}
impl EventEmitter<PriorityEvent> for PriorityState {}
impl Focusable for PriorityState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl PriorityState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), priority: ItemPriority::NONE }
    }

    pub fn priority(&self) -> ItemPriority {
        self.priority.clone()
    }

    pub fn set_priority(
        &mut self,
        date: impl Into<ItemPriority>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.priority = date.into();
        cx.notify()
    }

    fn on_action_info(
        &mut self,
        info: &PriorityInfo,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.priority = ItemPriority::from_i32(info.0);
        cx.emit(PriorityEvent::Selected(info.0));
        cx.notify();
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct PriorityButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<PriorityState>,
}

impl Sizable for PriorityButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for PriorityButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for PriorityButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for PriorityState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        v_flex().on_action(cx.listener(Self::on_action_info)).child(
            Button::new("priority")
                .outline()
                .icon(Icon::build(IconName::FlagOutlineThickSymbolic).text_color(
                    if self.priority == ItemPriority::NONE {
                        cx.theme().primary
                    } else {
                        gpui::rgb(self.priority.get_color()).into()
                    },
                ))
                .dropdown_menu_with_anchor(Corner::TopLeft, move |this, _, _| {
                    let mut this = this.scrollable(true).max_h(px(400.));
                    for p in ItemPriority::all() {
                        let p1 = p.clone() as i32;
                        this = this
                            .menu(SharedString::from(p.display_name()), Box::new(PriorityInfo(p1)))
                    }
                    this.min_w(px(100.))
                }),
        )
    }
}

impl PriorityButton {
    /// Create a new DatePicker with the given [`PriorityState`].
    pub fn new(state: &Entity<PriorityState>) -> Self {
        Self {
            id: ("item-priority", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for PriorityButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
