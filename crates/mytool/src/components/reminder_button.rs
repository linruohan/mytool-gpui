use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, FocusHandle, ParentElement, Render, Styled, Window,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::ReminderModel;

use crate::{
    components::{
        PopoverListMixin, PopoverSearchMixin, create_list_item_element, handle_search_input_change,
        manage_popover_state,
    },
    create_button_wrapper, impl_button_state_base,
    todo_actions::{add_reminder, delete_reminder},
};

pub type ReminderResult<T> = Result<T, ReminderError>;

#[derive(Debug, Clone)]
pub enum ReminderError {
    InvalidDate(String),
    InvalidTime(String),
    ParseError(String),
}

impl std::fmt::Display for ReminderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDate(msg) => write!(f, "Invalid date: {}", msg),
            Self::InvalidTime(msg) => write!(f, "Invalid time: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ReminderError {}

pub enum ReminderButtonEvent {
    Added(Arc<ReminderModel>),
    Removed(String),
    Error(Box<dyn std::error::Error + Send + Sync>),
}

pub struct ReminderButtonState {
    focus_handle: FocusHandle,
    pub item_id: String,
    search: PopoverSearchMixin,
    items: PopoverListMixin<Arc<ReminderModel>>,
    date_picker_state: Entity<DatePickerState>,
    current_date: String,
    current_time: String,
    show_date_picker: bool,
    show_time_dropdown: bool,
}

impl_button_state_base!(ReminderButtonState, ReminderButtonEvent);

impl ReminderButtonState {
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search reminders..."));

        // Subscribe to search events directly
        let _ = cx.subscribe_in(&search_input, window, Self::on_search_event);
        let _ = cx.subscribe_in(&date_picker_state, window, Self::on_date_picker_event);

        let filter_fn = |reminder: &Arc<ReminderModel>, query: &str| {
            reminder
                .due
                .as_ref()
                .map(|d| d.to_lowercase().contains(&query.to_lowercase()))
                .unwrap_or(false)
        };

        Self {
            focus_handle: cx.focus_handle(),
            item_id,
            search: PopoverSearchMixin::new(search_input),
            items: PopoverListMixin::new(filter_fn),
            date_picker_state,
            current_date: String::new(),
            current_time: "09:00".to_string(),
            show_date_picker: false,
            show_time_dropdown: false,
        }
    }

    pub fn set_reminders(&mut self, reminders: Vec<Arc<ReminderModel>>, cx: &mut Context<Self>) {
        // 检查是否有实际变化
        let old_reminders = self.items.items.clone();
        let has_changed = old_reminders.len() != reminders.len()
            || old_reminders.iter().zip(reminders.iter()).any(|(a, b)| a.id != b.id);

        self.items.set_items(reminders);

        // 只有在有实际变化时才通知UI刷新
        if has_changed {
            cx.notify();
        }
    }

    pub fn add_reminder(&mut self, reminder: Arc<ReminderModel>, cx: &mut Context<Self>) {
        self.items.add_item(reminder.clone());
        cx.emit(ReminderButtonEvent::Added(reminder));
        cx.notify();
    }

    pub fn remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.items.remove_item(|r| r.id == reminder_id);
        cx.emit(ReminderButtonEvent::Removed(reminder_id.to_string()));
        cx.notify();
    }

    fn on_search_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            handle_search_input_change(
                &self.search.search_input,
                &mut self.search.search_query,
                cx,
            );
        }
    }

    fn on_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        self.current_date = date.format("%Y-%m-%d").unwrap_or_default().to_string();
        self.show_date_picker = false;
        cx.notify();
    }

    fn on_time_select(&mut self, time: &str, cx: &mut Context<Self>) {
        self.current_time = time.to_string();
        self.show_time_dropdown = false;
        cx.notify();
    }

    fn get_time_options() -> Vec<&'static str> {
        vec!["09:00", "12:00", "17:30", "20:00"]
    }

    fn on_add_reminder(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.try_add_reminder(cx) {
            cx.emit(ReminderButtonEvent::Error(Box::new(e)));
        }
    }

    fn try_add_reminder(&mut self, cx: &mut Context<Self>) -> ReminderResult<()> {
        if self.current_date.is_empty() {
            return Err(ReminderError::InvalidDate("Date is required".to_string()));
        }

        let due_str = format!("{} {}:00", self.current_date, self.current_time);

        let reminder = ReminderModel {
            id: Uuid::new_v4().to_string(),
            item_id: Some(self.item_id.clone()),
            due: Some(due_str),
            reminder_type: Some("time".to_string()),
            ..Default::default()
        };

        self.add_reminder(Arc::new(reminder.clone()), cx);
        add_reminder(reminder, cx);

        self.current_date.clear();
        self.current_time = "09:00".to_string();
        Ok(())
    }

    fn on_remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.remove_reminder(reminder_id, cx);
        delete_reminder(reminder_id.to_string(), cx);
    }

    fn get_filtered_reminders(&self) -> Vec<Arc<ReminderModel>> {
        self.items.get_filtered(&self.search.search_query)
    }
}

impl Render for ReminderButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let show_date_picker = self.show_date_picker;
        let show_time_dropdown = self.show_time_dropdown;
        let date_picker_state = self.date_picker_state.clone();
        let search_input = self.search.search_input.clone();
        let filtered_reminders = self.get_filtered_reminders();

        Popover::new("reminder-popover")
            .p_0()
            .text_sm()
            .open(self.search.popover_open)
            .on_open_change(cx.listener(move |this, open, _, cx| {
                manage_popover_state(
                    &mut this.search.popover_open,
                    &mut this.search.search_query,
                    *open,
                );
                if !*open {
                    this.show_date_picker = false;
                    this.show_time_dropdown = false;
                }
                cx.notify();
            }))
            .trigger(
                Button::new("open-reminder-dialog").small().outline().icon(IconName::AlarmSymbolic),
            )
            .track_focus(&self.focus_handle)
            .child(
                v_flex()
                    .gap_3()
                    .p_3()
                    .w_96()
                    .child(Input::new(&search_input).flex_1())
                    // Date and Time Selection Section
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Button::new("date-picker-button")
                                            .small()
                                            .outline()
                                            .flex_1()
                                            .label(if self.current_date.is_empty() {
                                                "Select Date".to_string()
                                            } else {
                                                self.current_date.clone()
                                            })
                                            .on_click({
                                                let view = view.clone();
                                                move |_event, window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.show_date_picker = !this.show_date_picker;
                                                        if this.show_date_picker && this.current_date.is_empty() {
                                                            // Set today as default
                                                            let today = chrono::Utc::now().naive_utc().date();
                                                            this.date_picker_state.update(cx, |picker, cx| {
                                                                picker.set_date(today, window, cx);
                                                            });
                                                        }
                                                        cx.notify();
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("time-dropdown")
                                            .small()
                                            .outline()
                                            .label(&self.current_time)
                                            .on_click({
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.show_time_dropdown = !this.show_time_dropdown;
                                                        cx.notify();
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("add-reminder")
                                            .small()
                                            .primary()
                                            .icon(IconName::Plus)
                                            .on_click({
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_add_reminder(cx);
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Date Picker
                            .when(show_date_picker, {
                                let date_picker = date_picker_state.clone();
                                move |this| {
                                    this.child(
                                        DatePicker::new(&date_picker)
                                            .cleanable(true)
                                            .w(px(260.))
                                    )
                                }
                            })
                            // Time Dropdown
                            .when(show_time_dropdown, {
                                let view = view.clone();
                                move |this| {
                                    this.child(v_flex().gap_1().children(
                                        Self::get_time_options().iter().map(|time| {
                                            let view = view.clone();
                                            let time = *time;
                                            Button::new(format!("time-{}", time))
                                                .small()
                                                .label(time)
                                                .on_click({
                                                    let view = view.clone();
                                                    let time = time.to_string();
                                                    move |_event, _window, cx| {
                                                        cx.update_entity(&view, |this, cx| {
                                                            this.on_time_select(&time, cx);
                                                        });
                                                    }
                                                })
                                        }),
                                    ))
                                }
                            }),
                    )
                    // Reminders List
                    .child(v_flex().gap_2().children(filtered_reminders.iter().enumerate().map(
                        |(idx, reminder)| {
                            let reminder_id = reminder.id.clone();
                            let view = view.clone();
                            let display_text =
                                reminder.due.clone().unwrap_or_else(|| "No date".to_string());

                            create_list_item_element(
                                idx,
                                display_text,
                                reminder_id,
                                view,
                                move |item_id, view, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.on_remove_reminder(&item_id, cx);
                                    });
                                },
                            )
                        },
                    ))),
            )
    }
}

create_button_wrapper!(ReminderButton, ReminderButtonState, "item-reminder");
