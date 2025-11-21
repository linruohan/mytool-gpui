use std::rc::Rc;

use gpui::{
    Action, App, AppContext, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, ParentElement as _, Render, RenderOnce,
    SharedString, StyleRefinement, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    Sizable, Size, StyledExt as _, WindowExt,
    button::Button,
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    input::{Input, InputState},
    menu::DropdownMenu,
    v_flex,
};
use serde::Deserialize;
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};

use super::{PriorityButton, PriorityEvent, PriorityState};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = item_info, no_json)]
struct Info(i32);
const CONTEXT: &'static str = "ItemInfo";
#[derive(Clone)]
pub enum ItemInfoEvent {
    Update(Rc<ItemModel>),
    Add(Rc<ItemModel>),
}
pub struct ItemInfoState {
    focus_handle: FocusHandle,
    pub item: Rc<ItemModel>,
    _subscriptions: Vec<Subscription>,
    // item view
    checked: bool,
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    date: Entity<DatePickerState>,
    priority_state: Entity<PriorityState>,
}

impl Focusable for ItemInfoState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemInfoEvent> for ItemInfoState {}

impl ItemInfoState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Rc::new(ItemModel::default());
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("To-do Name"));
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("Add a description ...")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        let priority_state = cx.new(|cx| PriorityState::new(window, cx));

        let _subscriptions = vec![cx.subscribe_in(
            &priority_state,
            window,
            move |this, _, ev: &PriorityEvent, _window, _cx| match ev {
                PriorityEvent::Selected(priority) => {
                    this.set_priority(*priority);
                },
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            item,
            _subscriptions,
            name_input,
            desc_input,
            checked: false,
            date,
            priority_state,
        }
    }

    pub fn priority(&self) -> Option<ItemPriority> {
        Some(ItemPriority::from_i32(self.item.priority.unwrap_or_default()))
    }

    pub fn set_priority(&mut self, priority: i32) {
        let item = Rc::make_mut(&mut self.item);
        item.priority = Some(priority);
    }

    /// Set the date of the date picker.
    pub fn set_item(
        &mut self,
        item: Rc<ItemModel>,
        _emit: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.item = item;
    }

    pub fn handel_item_info_event(&mut self, event: &ItemInfoEvent, cx: &mut Context<Self>) {
        match event {
            ItemInfoEvent::Add(item) => {
                self.update_item(item.clone(), cx);
            },
            ItemInfoEvent::Update(item) => self.update_item(item.clone(), cx),
        }
    }

    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.checked = *selectable;
    }

    fn update_item(&mut self, item: Rc<ItemModel>, _cx: &mut Context<Self>) {
        self.item = item.clone();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        v_flex()
            .child(
                Checkbox::new("item-is-finish")
                    .checked(self.checked)
                    .on_click(cx.listener(Self::toggle_finished)),
            )
            .child(Input::new(&self.name_input))
            .child(Input::new(&self.desc_input))
            .child(
                h_flex().child(PriorityButton::new(&self.priority_state)).child(
                    Button::new("dropdown-menu-scrollable-2")
                        .outline()
                        .label("Scrollable Menu (5 items)")
                        .dropdown_menu_with_anchor(Corner::TopLeft, move |this, _, _| {
                            let mut this = this
                                .scrollable(true)
                                .max_h(px(300.))
                                .label(format!("Total {} items", 100));
                            for i in 0..5 {
                                this = this.menu(
                                    SharedString::from(format!("Item {}", i)),
                                    Box::new(Info(i)),
                                )
                            }
                            this.min_w(px(100.))
                        }),
                ),
            )
            .child(DatePicker::new(&self.date).placeholder("Date of Birth"))
            .child(Button::new("12").label("获取item").on_click({
                let view = view.clone();
                let name_input_clone1 = self.name_input.clone();
                let des_input_clone1 = self.desc_input.clone();
                move |_, window, cx| {
                    window.close_dialog(cx);
                    view.update(cx, |view, cx| {
                        let item = ItemModel {
                            content: name_input_clone1.read(cx).value().to_string(),
                            description: Some(des_input_clone1.read(cx).value().to_string()),
                            checked: view.checked,
                            priority: Some(view.priority_state.read(cx).priority() as i32),
                            ..Default::default()
                        };
                        cx.emit(ItemInfoEvent::Update(item.into()));
                        cx.notify();
                    });
                }
            }))
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct ItemInfo {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ItemInfoState>,
}

impl Sizable for ItemInfo {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for ItemInfo {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ItemInfo {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ItemInfo {
    /// Create a new DatePicker with the given [`ItemInfoState`].
    pub fn new(state: &Entity<ItemInfoState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ItemInfo {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            // .flex_none()
            .w_full()
            // .relative()
            // .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
