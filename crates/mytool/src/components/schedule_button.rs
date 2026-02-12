use chrono::Local;
use gpui::{
    Action, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Selectable, Sizable, StyleSized,
    button::Button,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    divider::Divider,
    h_flex,
    label::Label,
    number_input::NumberInput,
    popover::Popover,
    space::Space,
    text_input::TextInput,
    v_flex,
};
use serde::Deserialize;
use todos::{DueDate, enums::RecurrencyType};

use crate::{create_button_wrapper, impl_button_state_base};

// Actions for the schedule button
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
enum ScheduleAction {
    SetDatePreset(String),
    OpenDatePicker,
    SetRecurrence(String),
    SetCustomRecurrenceType(String),
    SetCustomRecurrenceInterval(i64),
    SetEndType(String),
    SetEndDate(String),
    SetEndCount(i64),
    ToggleTimeInput,
    SetTime(String),
    ClearSchedule,
    ConfirmSchedule,
}

// Events emitted by the schedule button
pub enum ScheduleButtonEvent {
    DateSelected(String),
    TimeSelected(String),
    RecurrenceSelected(RecurrencyType),
    Cleared,
}

// State for the schedule button
pub struct ScheduleButtonState {
    focus_handle: FocusHandle,
    due_date: DueDate,
    date_picker_state: Entity<DatePickerState>,
    end_date_picker_state: Entity<DatePickerState>,
    popover_open: bool,
    show_date_picker: bool,
    show_time_input: bool,
    show_custom_recurrence: bool,
    custom_recurrence_type: String,
    custom_recurrence_interval: i64,
    end_type: String, // never, on_date, after
    _subscriptions: Vec<Subscription>,
}

impl_button_state_base!(ScheduleButtonState, ScheduleButtonEvent);

impl ScheduleButtonState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let end_date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let _subscriptions = vec![
            cx.subscribe_in(&date_picker_state, window, Self::on_date_picker_event),
            cx.subscribe_in(&end_date_picker_state, window, Self::on_end_date_picker_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            date_picker_state,
            end_date_picker_state,
            popover_open: false,
            show_date_picker: false,
            show_time_input: false,
            show_custom_recurrence: false,
            custom_recurrence_type: "days".to_string(),
            custom_recurrence_interval: 1,
            end_type: "never".to_string(),
            _subscriptions,
        }
    }

    // Get the due date
    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    // Set the due date
    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        if self.due_date != due_date {
            self.due_date = due_date.clone();

            // Initialize UI state based on due_date
            self.show_time_input = !due_date.date.is_empty() && due_date.date.contains(' ');
            self.show_custom_recurrence = due_date.is_recurring
                && matches!(
                    due_date.recurrency_type,
                    RecurrencyType::MINUTELY
                        | RecurrencyType::HOURLY
                        | RecurrencyType::EveryDay
                        | RecurrencyType::EveryWeek
                        | RecurrencyType::EveryMonth
                        | RecurrencyType::EveryYear
                );
            self.custom_recurrence_type = match due_date.recurrency_type {
                RecurrencyType::MINUTELY => "minutes".to_string(),
                RecurrencyType::HOURLY => "hours".to_string(),
                RecurrencyType::EveryDay => "days".to_string(),
                RecurrencyType::EveryWeek => "weeks".to_string(),
                RecurrencyType::EveryMonth => "months".to_string(),
                RecurrencyType::EveryYear => "years".to_string(),
                _ => "days".to_string(),
            };
            self.custom_recurrence_interval = due_date.recurrency_interval;
            self.end_type = if due_date.recurrency_count > 0 {
                "after".to_string()
            } else if !due_date.recurrency_end.is_empty() {
                "on_date".to_string()
            } else {
                "never".to_string()
            };

            if let Some(dt) = due_date.datetime() {
                let date = dt.date();
                self.date_picker_state.update(cx, |picker, cx| {
                    picker.set_date(date, window, cx);
                });
            }
            cx.notify();
        }
    }

    // Handle date picker events
    fn on_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let DatePickerEvent::Change(date) = event {
            if let Some(date_str) = date.format("%Y-%m-%d") {
                let time_str = self.get_current_time();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                self.show_date_picker = false;
                cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
                cx.notify();
            }
        }
    }

    // Handle end date picker events
    fn on_end_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let DatePickerEvent::Change(date) = event {
            if let Some(date_str) = date.format("%Y-%m-%d") {
                self.due_date.recurrency_end = date_str.to_string();
                cx.notify();
            }
        }
    }

    // Handle schedule actions
    fn on_schedule_action(
        &mut self,
        action: &ScheduleAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ScheduleAction::SetDatePreset(preset) => {
                let today = Local::now().naive_local().date();
                let date = match preset.as_str() {
                    "today" => today,
                    "tomorrow" => today.succ_opt().unwrap_or(today),
                    "next_week" => today + chrono::Duration::days(7),
                    _ => return,
                };
                let date_str = date.format("%Y-%m-%d").to_string();
                let time_str = self.get_current_time();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                self.date_picker_state.update(cx, |picker, cx| {
                    picker.set_date(date, window, cx);
                });
                cx.emit(ScheduleButtonEvent::DateSelected(preset.to_string()));
                cx.notify();
            },
            ScheduleAction::OpenDatePicker => {
                self.show_date_picker = true;
                if self.due_date.date.is_empty() {
                    let today = Local::now().naive_local().date();
                    self.date_picker_state.update(cx, |picker, cx| {
                        picker.set_date(today, window, cx);
                    });
                }
                cx.notify();
            },
            ScheduleAction::SetRecurrence(option) => {
                // Reset all recurrence fields first
                self.due_date.recurrency_weeks = "".to_string();
                self.due_date.recurrency_count = 0;
                self.due_date.recurrency_end = "".to_string();

                match option.as_str() {
                    "daily" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryDay;
                        self.due_date.recurrency_interval = 1;
                        self.show_custom_recurrence = false;
                    },
                    "weekdays" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                        self.due_date.recurrency_interval = 1;
                        self.due_date.recurrency_weeks = "1,2,3,4,5".to_string();
                        self.show_custom_recurrence = false;
                    },
                    "weekends" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                        self.due_date.recurrency_interval = 1;
                        self.due_date.recurrency_weeks = "0,6".to_string();
                        self.show_custom_recurrence = false;
                    },
                    "weekly" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                        self.due_date.recurrency_interval = 1;
                        self.show_custom_recurrence = false;
                    },
                    "monthly" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryMonth;
                        self.due_date.recurrency_interval = 1;
                        self.show_custom_recurrence = false;
                    },
                    "yearly" => {
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                        self.due_date.recurrency_type = RecurrencyType::EveryYear;
                        self.due_date.recurrency_interval = 1;
                        self.show_custom_recurrence = false;
                    },
                    "none" => {
                        self.due_date.is_recurring = false;
                        self.due_date.recurrency_supported = false;
                        self.due_date.recurrency_type = RecurrencyType::NONE;
                        self.due_date.recurrency_interval = 0;
                        self.show_custom_recurrence = false;
                    },
                    "custom" => {
                        self.show_custom_recurrence = true;
                        self.due_date.is_recurring = true;
                        self.due_date.recurrency_supported = true;
                    },
                    _ => {},
                }
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(
                    self.due_date.recurrency_type.clone(),
                ));
                cx.notify();
            },
            ScheduleAction::SetCustomRecurrenceType(rec_type) => {
                self.custom_recurrence_type = rec_type.clone();
                match rec_type.as_str() {
                    "minutes" => self.due_date.recurrency_type = RecurrencyType::MINUTELY,
                    "hours" => self.due_date.recurrency_type = RecurrencyType::HOURLY,
                    "days" => self.due_date.recurrency_type = RecurrencyType::EveryDay,
                    "weeks" => self.due_date.recurrency_type = RecurrencyType::EveryWeek,
                    "months" => self.due_date.recurrency_type = RecurrencyType::EveryMonth,
                    "years" => self.due_date.recurrency_type = RecurrencyType::EveryYear,
                    _ => {},
                }
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(
                    self.due_date.recurrency_type.clone(),
                ));
                cx.notify();
            },
            ScheduleAction::SetCustomRecurrenceInterval(interval) => {
                self.custom_recurrence_interval = *interval;
                self.due_date.recurrency_interval = *interval;
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(
                    self.due_date.recurrency_type.clone(),
                ));
                cx.notify();
            },
            ScheduleAction::SetEndType(end_type) => {
                self.end_type = end_type.clone();
                match end_type.as_str() {
                    "never" => {
                        self.due_date.recurrency_count = 0;
                        self.due_date.recurrency_end = "".to_string();
                    },
                    "on_date" => {
                        // End date will be set separately
                    },
                    "after" => {
                        self.due_date.recurrency_end = "".to_string();
                        // End count will be set separately
                    },
                    _ => {},
                }
                cx.notify();
            },
            ScheduleAction::SetEndDate(date_str) => {
                self.due_date.recurrency_end = date_str.clone();
                cx.notify();
            },
            ScheduleAction::SetEndCount(count) => {
                self.due_date.recurrency_count = *count;
                cx.notify();
            },
            ScheduleAction::ToggleTimeInput => {
                self.show_time_input = !self.show_time_input;
                cx.notify();
            },
            ScheduleAction::SetTime(time) => {
                if let Some((date_part, _)) = self.due_date.date.split_once(' ') {
                    self.due_date.date = format!("{} {}:00", date_part, time);
                    cx.emit(ScheduleButtonEvent::TimeSelected(time.to_string()));
                    cx.notify();
                }
            },
            ScheduleAction::ClearSchedule => {
                self.due_date = DueDate::default();
                self.show_date_picker = false;
                self.show_time_input = false;
                self.show_custom_recurrence = false;
                self.end_type = "never".to_string();
                cx.emit(ScheduleButtonEvent::Cleared);
                cx.notify();
            },
            ScheduleAction::ConfirmSchedule => {
                self.popover_open = false;
                self.show_date_picker = false;
                cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
                cx.notify();
            },
        }
    }

    // Get display text for the button
    fn get_display_text(&self) -> String {
        if self.due_date.date.is_empty() {
            "Schedule".to_string()
        } else {
            let today = Local::now().naive_local().date();
            if let Some(dt) = self.due_date.datetime() {
                let date = dt.date();
                if date == today {
                    "Today".to_string()
                } else if date == today.succ_opt().unwrap_or(today) {
                    "Tomorrow".to_string()
                } else {
                    date.format("%b %d").to_string()
                }
            } else {
                "Schedule".to_string()
            }
        }
    }

    // Get current time in HH:MM format
    fn get_current_time(&self) -> String {
        if let Some(dt) = self.due_date.datetime() {
            dt.time().format("%H:%M").to_string()
        } else {
            "17:30".to_string() // Default time
        }
    }
}

impl_button_state_base!(ScheduleButtonState, ScheduleButtonEvent);

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let date_picker_state = self.date_picker_state.clone();
        let end_date_picker_state = self.end_date_picker_state.clone();

        v_flex()
            .on_action(cx.listener(Self::on_schedule_action))
            .child(
                Popover::new(("schedule-popover", cx.entity_id()))
                    .open(self.popover_open)
                    .on_open_change(move |open, cx| {
                        cx.update(|state, _| {
                            state.popover_open = open;
                        });
                    })
                    .trigger(
                        Button::new(("schedule-button", cx.entity_id()))
                            .outline()
                            .tooltip("Set schedule")
                            .icon(IconName::Calendar)
                            .label(SharedString::from(self.get_display_text()))
                    )
                    .content(move |_, window, cx| {
                        let date_picker = date_picker_state.clone();
                        let end_date_picker = end_date_picker_state.clone();

                        v_flex()
                            .min_w(px(320.))
                            // Date Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        h_flex()
                                            .gap(px(6.))
                                            .child(
                                                Button::new(("today-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Today"))
                                                    .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetDatePreset("today".to_string()), window, cx); }); } })
                                            )
                                            .child(
                                                Button::new(("tomorrow-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Tomorrow"))
                                                    .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetDatePreset("tomorrow".to_string()), window, cx); }); } })
                                            )
                                            .child(
                                                Button::new(("next-week-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Next Week"))
                                                    .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetDatePreset("next_week".to_string()), window, cx); }); } })
                                            )
                                            .child(
                                                Button::new(("custom-date-btn", cx.entity_id()))
                                                    .outline()
                                                    .small()
                                                    .label(SharedString::from("Custom"))
                                                    .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::OpenDatePicker, window, cx); }); } })
                                            )
                                    )
                                    .when(self.show_date_picker, move |this| {
                                        this.child(Space::new(px(8.)))
                                            .child(
                                                DatePicker::new(&date_picker)
                                                    .cleanable(true)
                                                    .w(px(260.))
                                            )
                                    })
                            )
                            .child(Divider::horizontal())
                            // Recurrence Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        v_flex()
                                            .gap(px(6.))
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .child(
                                                        Button::new(("daily-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Daily"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryDay && self.due_date.recurrency_interval == 1)
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("daily".to_string()), window, cx); }); } })
                                                    )
                                                    .child(
                                                        Button::new(("weekdays-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekdays"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryWeek && self.due_date.recurrency_weeks == "1,2,3,4,5")
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("weekdays".to_string()), window, cx); }); } })
                                                    )
                                                    .child(
                                                        Button::new(("weekends-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekends"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryWeek && self.due_date.recurrency_weeks == "0,6")
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("weekends".to_string()), window, cx); }); } })
                                                    )
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .child(
                                                        Button::new(("weekly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekly"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryWeek && self.due_date.recurrency_interval == 1 && self.due_date.recurrency_weeks.is_empty())
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("weekly".to_string()), window, cx); }); } })
                                                    )
                                                    .child(
                                                        Button::new(("monthly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Monthly"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryMonth && self.due_date.recurrency_interval == 1)
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("monthly".to_string()), window, cx); }); } })
                                                    )
                                                    .child(
                                                        Button::new(("yearly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Yearly"))
                                                            .selected(self.due_date.is_recurring && self.due_date.recurrency_type == RecurrencyType::EveryYear && self.due_date.recurrency_interval == 1)
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("yearly".to_string()), window, cx); }); } })
                                                    )
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .child(
                                                        Button::new(("none-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("None"))
                                                            .selected(!self.due_date.is_recurring)
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("none".to_string()), window, cx); }); } })
                                                    )
                                                    .child(
                                                        Button::new(("custom-recurrence-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Custom"))
                                                            .selected(self.show_custom_recurrence)
                                                            .on_click({ let view = cx.entity(); move |_, window, cx| { cx.update_entity(&view, |this, cx| { this.on_schedule_action(&ScheduleAction::SetRecurrence("custom".to_string()), window, cx); }); } })
                                                    )
                                            )
                                    )
                                    .when(self.show_custom_recurrence, |this: &mut gpui_component::v_flex::VFlex| {
                                        this.child(Space::new(px(12.)))
                                            .child(
                                                v_flex()
                                                    .p(px(12.))
                                                    .bg(cx.theme().background.surface_1)
                                                    .border_r(px(8.))
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .child(
                                                                Label::new("Repeat every")
                                                                    .list_size(gpui_component::Size::Size(px(12.)))
                                                                    .mr(px(8.))
                                                            )
                                                            .child(
                                                                NumberInput::new(("interval-input", cx.entity_id()))
                                                                    .value(self.custom_recurrence_interval)
                                                                    .min(1)
                                                                    .max(999)
                                                                    .w(px(60.))
                                                                    .on_change(cx.action(|value: i64| ScheduleAction::SetCustomRecurrenceInterval(value)))
                                                            )
                                                            .child(
                                                                h_flex()
                                                                    .gap(px(2.))
                                                                    .child(
                                                                        Button::new(("minutes-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("minutes"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("minutes".to_string())))
                                                                            .selected(self.custom_recurrence_type == "minutes")
                                                                    )
                                                                    .child(
                                                                        Button::new(("hours-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("hours"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("hours".to_string())))
                                                                            .selected(self.custom_recurrence_type == "hours")
                                                                    )
                                                                    .child(
                                                                        Button::new(("days-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("days"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("days".to_string())))
                                                                            .selected(self.custom_recurrence_type == "days")
                                                                    )
                                                                    .child(
                                                                        Button::new(("weeks-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("weeks"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("weeks".to_string())))
                                                                            .selected(self.custom_recurrence_type == "weeks")
                                                                    )
                                                                    .child(
                                                                        Button::new(("months-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("months"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("months".to_string())))
                                                                            .selected(self.custom_recurrence_type == "months")
                                                                    )
                                                                    .child(
                                                                        Button::new(("years-btn", cx.entity_id()))
                                                                            .small()
                                                                            .label(SharedString::from("years"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("years".to_string())))
                                                                            .selected(self.custom_recurrence_type == "years")
                                                                    )
                                                            )
                                                    )
                                                    .child(Space::new(px(12.)))
                                                    .child(
                                                        Label::new("End Repeat")
                                                            .list_size(gpui_component::Size::Size(px(12.)))
                                                            
                                                            .mb(px(8.))
                                                    )
                                                    .child(
                                                        h_flex()
                                                            .gap(px(6.))
                                                            .child(
                                                                Button::new(("never-btn", cx.entity_id()))
                                                                    .small()
                                                                    .label(SharedString::from("Never"))
                                                                    .on_click(cx.action(ScheduleAction::SetEndType("never".to_string())))
                                                                    .selected(self.end_type == "never")
                                                            )
                                                            .child(
                                                                Button::new(("on-date-btn", cx.entity_id()))
                                                                    .small()
                                                                    .label(SharedString::from("On Date"))
                                                                    .on_click(cx.action(ScheduleAction::SetEndType("on_date".to_string())))
                                                                    .selected(self.end_type == "on_date")
                                                            )
                                                            .child(
                                                                Button::new(("after-btn", cx.entity_id()))
                                                                    .small()
                                                                    .label(SharedString::from("After"))
                                                                    .on_click(cx.action(ScheduleAction::SetEndType("after".to_string())))
                                                                    .selected(self.end_type == "after")
                                                            )
                                                    )
                                                    .when(self.end_type == "on_date", |this: &mut gpui_component::v_flex::VFlex| {
                                                        this.child(Space::new(px(8.)))
                                                            .child(
                                                                DatePicker::new(&date_picker)
                                                                    .cleanable(true)
                                                                    .w(px(200.))
                                                                    .on_date_change(cx.action(|date| {
                                                                        if let Some(date) = date {
                                                                            ScheduleAction::SetEndDate(date.format("%Y-%m-%d").to_string())
                                                                        } else {
                                                                            ScheduleAction::SetEndDate("".to_string())
                                                                        }
                                                                    }))
                                                            )
                                                    })
                                                    .when(self.end_type == "after", |this: &mut gpui_component::v_flex::VFlex| {
                                                        this.child(Space::new(px(8.)))
                                                            .child(
                                                                h_flex()
                                                                    .items_center()
                                                                    .child(
                                                                        NumberInput::new(("end-count-input", cx.entity_id()))
                                                                            .value(self.due_date.recurrency_count.max(1))
                                                                            .min(1)
                                                                            .max(999)
                                                                            .w(px(80.))
                                                                            .on_change(cx.action(|value: i64| ScheduleAction::SetEndCount(value)))
                                                                    )
                                                                    .child(
                                                                        Label::new("occurrences")
                                                                            .list_size(gpui_component::Size::Size(px(12.)))
                                                                            .ml(px(8.))
                                                                    )
                                                            )
                                                    })
                                            )
                                    })
                            )
                            .child(Divider::horizontal())
                            // Time Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .child(
                                                Label::new("Time:")
                                                    .list_size(gpui_component::Size::Size(px(14.)))
                                                    
                                                    .mr(px(8.))
                                            )
                                            .when(!self.show_time_input, |this: &mut gpui_component::h_flex::HFlex| {
                                                this.child(
                                                    Button::new(("time-toggle", cx.entity_id()))
                                                        .small()
                                                        .icon(IconName::Plus)
                                                        .on_click(cx.action(ScheduleAction::ToggleTimeInput))
                                                )
                                            })
                                            .when(!self.show_time_input, |this: &mut gpui_component::h_flex::HFlex| {
                                                this.child(
                                                    Label::new("None")
                                                        .list_size(gpui_component::Size::Size(px(12.)))
                                                        .text_color(cx.theme().foreground.muted)
                                                        .ml(px(8.))
                                                )
                                            })
                                            .when(self.show_time_input, |this: &mut gpui_component::v_flex::VFlex| {
                        this.child(Space::new(px(8.)))
                            .child(
                                TextInput::new(("time-input", cx.entity_id()))
                                    .placeholder(SharedString::from("10:12"))
                                    .value(SharedString::from(self.get_current_time()))
                                    .w(px(100.))
                                    .on_change(cx.action(|value: gpui::SharedString| ScheduleAction::SetTime(value.to_string())))
                            )
                    })
                                    )
                            )
                            .child(Divider::horizontal())
                            // Footer Buttons
                            .child(
                                h_flex()
                                    .p(px(16.))
                                    .content_end()
                                    .gap(px(8.))
                                    .child(
                                        Button::new(("clear-btn", cx.entity_id()))
                                            .outline()
                                            .small()
                                            .label(SharedString::from("Clear"))
                                            .on_click(cx.action(ScheduleAction::ClearSchedule))
                                    )
                                    .child(
                                        Button::new(("done-btn", cx.entity_id()))
                                            .small()
                                            .label(SharedString::from("Done"))
                                            .on_click(cx.action(ScheduleAction::ConfirmSchedule))
                                    )
                            )
                    })
            )
    }
}

// Create the button wrapper
create_button_wrapper!(ScheduleButton, ScheduleButtonState, "item-schedule");
