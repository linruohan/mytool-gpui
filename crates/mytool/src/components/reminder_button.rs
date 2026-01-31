use std::rc::Rc;

use chrono::Local;
use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex, v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::ReminderModel;

use crate::todo_actions::{add_reminder, delete_reminder};

pub enum ReminderButtonEvent {
    Added(Rc<ReminderModel>),
    Removed(String), // reminder id
}

pub struct ReminderButtonState {
    focus_handle: FocusHandle,
    pub reminders: Vec<Rc<ReminderModel>>,
    pub item_id: String,
    date_picker_state: Entity<DatePickerState>,
    show_date_picker: bool,
    selected_time: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ReminderButtonEvent> for ReminderButtonState {}

impl Focusable for ReminderButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ReminderButtonState {
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_event)];

        Self {
            focus_handle: cx.focus_handle(),
            reminders: Vec::new(),
            item_id,
            date_picker_state,
            show_date_picker: false,
            selected_time: None,
            _subscriptions,
        }
    }

    pub fn set_reminders(&mut self, reminders: Vec<Rc<ReminderModel>>, cx: &mut Context<Self>) {
        self.reminders = reminders;
        cx.notify();
    }

    pub fn add_reminder(&mut self, reminder: Rc<ReminderModel>, cx: &mut Context<Self>) {
        self.reminders.push(reminder.clone());
        cx.emit(ReminderButtonEvent::Added(reminder));
        cx.notify();
    }

    pub fn remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.reminders.retain(|r| r.id != reminder_id);
        cx.emit(ReminderButtonEvent::Removed(reminder_id.to_string()));
        cx.notify();
    }

    fn on_date_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        if let Some(date_str) = date.format("%Y-%m-%d").map(|s| s.to_string()) {
            let time_str = self.selected_time.clone().unwrap_or_else(|| "09:00".to_string());
            let due_str = format!("{} {}:00", date_str, time_str);

            // 创建 ReminderModel
            let reminder = ReminderModel {
                id: Uuid::new_v4().to_string(),
                item_id: Some(self.item_id.clone()),
                due: Some(due_str),
                reminder_type: Some("time".to_string()),
                ..Default::default()
            };

            self.add_reminder(Rc::new(reminder.clone()), cx);

            // 保存到数据库
            add_reminder(reminder, cx);

            self.show_date_picker = false;
            cx.notify();
        }
    }

    fn on_select_time(&mut self, time: &str, cx: &mut Context<Self>) {
        self.selected_time = Some(time.to_string());
        cx.notify();
    }

    fn on_show_date_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_date_picker = true;
        let today = Local::now().naive_local().date();
        self.date_picker_state.update(cx, |picker, cx| {
            picker.set_date(today, window, cx);
        });
        cx.notify();
    }

    fn on_remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.remove_reminder(reminder_id, cx);

        // 从数据库删除
        delete_reminder(reminder_id.to_string(), cx);
    }

    fn get_reminder_display(&self, reminder: &ReminderModel) -> String {
        reminder.due.clone().unwrap_or_else(|| "No date".to_string())
    }
}

impl Render for ReminderButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let date_picker = self.date_picker_state.clone();
        let show_date_picker = self.show_date_picker;

        v_flex()
            .gap_2()
            .child(
                h_flex().gap_2().items_center().child(
                    Button::new("add-reminder")
                        .small()
                        .outline()
                        .icon(IconName::AlarmSymbolic)
                        .on_click({
                            let view = view.clone();
                            move |_event, window, cx| {
                                cx.update_entity(&view, |this, cx| {
                                    this.on_show_date_picker(window, cx);
                                });
                            }
                        }),
                ),
            )
            .when(show_date_picker, {
                let view = view.clone();
                move |this| {
                    this.child(
                        v_flex()
                            .gap_2()
                            .child(DatePicker::new(&date_picker).cleanable(true).w(px(260.)))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("time-09:00").small().label("09:00").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_select_time("09:00", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new("time-12:00").small().label("12:00").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_select_time("12:00", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new("time-17:30").small().label("17:30").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_select_time("17:30", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    ),
                            ),
                    )
                }
            })
            .children(self.reminders.iter().enumerate().map(|(idx, reminder)| {
                let reminder_id = reminder.id.clone();
                let view = view.clone();

                h_flex()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .border_1()
                    .rounded(px(4.0))
                    .child(
                        gpui_component::label::Label::new(self.get_reminder_display(reminder))
                            .text_sm(),
                    )
                    .child(
                        Button::new(format!("remove-reminder-{}", idx))
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::UserTrashSymbolic)
                            .on_click({
                                let reminder_id = reminder_id.clone();
                                let view = view.clone();
                                move |_event, _window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.on_remove_reminder(&reminder_id, cx);
                                    });
                                }
                            }),
                    )
            }))
    }
}

#[derive(IntoElement)]
pub struct ReminderButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ReminderButtonState>,
}

impl Sizable for ReminderButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ReminderButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ReminderButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ReminderButton {
    pub fn new(state: &Entity<ReminderButtonState>) -> Self {
        Self {
            id: ("item-reminder", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ReminderButton {
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
