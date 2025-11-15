use std::rc::Rc;

use gpui::{div, Action, App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement, ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window};
use gpui_component::checkbox::Checkbox;
use gpui_component::{button::Button, date_picker::{DatePicker, DatePickerState}, input::{Input, InputState}, v_flex, Disableable, Sizable, Size, StyleSized as _, StyledExt as _, WindowExt};
use serde::Deserialize;
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};

use crate::components::priority_button::{PriorityButton, PriorityState};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = item_info, no_json)]
struct Info(i32);
const CONTEXT: &'static str = "ItemInfo";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        // KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        // KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        // KeyBinding::new("delete", Delete, Some(CONTEXT)),
        // KeyBinding::new("backspace", Delete, Some(CONTEXT)),
    ])
}

/// Events emitted by the DatePicker.
#[derive(Clone)]
pub enum ItemInfoEvent {
    Update(Rc<ItemModel>),
    Add(Rc<ItemModel>),
}
pub struct ItemInfoState {
    focus_handle: FocusHandle,
    item: Rc<ItemModel>,
    _subscriptions: Vec<Subscription>,
    // item view
    checked: bool,
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    date: Entity<DatePickerState>,
    priority_button: Entity<PriorityState>,
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
        let priority_button = cx.new(|cx| PriorityState::new(window, cx));

        let _subscriptions = vec![
            //     cx.subscribe_in(&calendar, window, |this, _, ev: &CalendarEvent, window, cx| {
            //     match ev {
            //         CalendarEvent::Selected(date) => {
            //             this.update_item(*date, true, window, cx);
            //             this.focus_handle.focus(window);
            //         },
            //     }
            // })
        ];

        Self {
            focus_handle: cx.focus_handle(),
            item,
            _subscriptions,
            name_input,
            desc_input,
            checked: false,
            date,
            priority_button,
        }
    }

    pub fn priority(&self) -> Option<ItemPriority> {
        Some(ItemPriority::from_i32(self.item.priority.clone().unwrap_or_default()))
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

    fn on_action_info(&mut self, info: &Info, _: &mut Window, _cx: &mut Context<Self>) {
        println!("info:{}", info.0);
        println!("item:{:?}", self.item);
    }
    pub fn handel_item_info_event(&mut self, event: &ItemInfoEvent, cx: &mut Context<Self>) {
        match event {
            ItemInfoEvent::Add(item) => {
                self.update_item(item.clone(), cx);
            }
            ItemInfoEvent::Update(item) => {
                self.update_item(item.clone(), cx)
            }
        }
    }
    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.checked = *selectable;
    }
    fn update_item(
        &mut self,
        item: Rc<ItemModel>,
        cx: &mut Context<Self>,
    ) {
        self.item = item.clone();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        v_flex()
            .on_action(cx.listener(Self::on_action_info))
            .child(Checkbox::new("item-is-finish").checked(self.checked).on_click(cx.listener(Self::toggle_finished)))
            .child(Input::new(&self.name_input))
            .child(Input::new(&self.desc_input))
            .child(PriorityButton::new(&self.priority_button))
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
                            description: Some(
                                des_input_clone1.read(cx).value().to_string(),
                            ),
                            checked: view.checked,
                            priority: Some(view.priority_button.read(cx).priority() as i32),
                            ..Default::default()
                        };
                        println!("emit ItemInfoEvent before:{:?}", item.clone());
                        cx.emit(ItemInfoEvent::Update(item.into()));
                        cx.notify();
                    });
                }
            }
            ))
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct ItemInfo {
    id: ElementId,
    style: StyleRefinement,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
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

impl Disableable for ItemInfo {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ItemInfo {
    /// Create a new DatePicker with the given [`ItemInfoState`].
    pub fn new(state: &Entity<ItemInfoState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            appearance: true,
            disabled: false,
        }
    }

    /// Set the placeholder of the date picker, default: "".
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set appearance of the date picker, if false, the date picker will be in a minimal style.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for ItemInfo {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
