use chrono::Local;
use gpui::{Action, Context, Entity, FocusHandle, Render, SharedString, Subscription, Window, prelude::FluentBuilder, px};
use gpui_component::{IconName, button::Button, date_picker::{DatePicker, DatePickerEvent, DatePickerState}, popover::Popover, number_input::NumberInput, v_flex, h_flex, text_input::TextInput, label::Label, space::Space, divider::Divider};
use serde::Deserialize;
use todos::{DueDate, enums::RecurrencyType};

use crate::{create_complex_button, impl_button_state_base};

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
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_picker_event)];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            date_picker_state,
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
            self.due_date = due_date;
            if let Some(dt) = self.due_date.datetime() {
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
            let date_str = date.format("%Y-%m-%d").to_string();
            let time_str = self.get_current_time();
            self.due_date.date = format!("{} {}:00", date_str, time_str);
            self.show_date_picker = false;
            cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
            cx.notify();
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
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(self.due_date.recurrency_type.clone()));
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
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(self.due_date.recurrency_type.clone()));
                cx.notify();
            },
            ScheduleAction::SetCustomRecurrenceInterval(interval) => {
                self.custom_recurrence_interval = *interval;
                self.due_date.recurrency_interval = *interval;
                cx.emit(ScheduleButtonEvent::RecurrenceSelected(self.due_date.recurrency_type.clone()));
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

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let date_picker_state = self.date_picker_state.clone();

        v_flex()
            .on_action(cx.listener(Self::on_schedule_action))
            .child(
                Popover::new(("schedule-popover", cx.entity_id()))
                    .open(self.popover_open)
                    .on_open_change(cx.callback(|open, cx| {
                        cx.update(|state, _| {
                            state.popover_open = open;
                        });
                    }))
                    .trigger(
                        Button::new(("schedule-button", cx.entity_id()))
                            .outline()
                            .tooltip("Set schedule")
                            .icon(IconName::Calendar)
                            .label(SharedString::from(self.get_display_text()))
                    )
                    .content(move |window, cx| {
                        let date_picker = date_picker_state.clone();

                        v_flex()
                            .min_w(px(320.))
                            // Date Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        Label::new("Date")
                                            .font_size(px(14.))
                                            .font_weight(500)
                                            .mb(px(8.))
                                    )
                                    .child(
                                        h_flex()
                                            .space_between()
                                            .child(
                                                Button::new(("today-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Today"))
                                                    .on_click(cx.action(ScheduleAction::SetDatePreset("today".to_string())))
                                            )
                                            .child(
                                                Button::new(("tomorrow-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Tomorrow"))
                                                    .on_click(cx.action(ScheduleAction::SetDatePreset("tomorrow".to_string())))
                                            )
                                            .child(
                                                Button::new(("next-week-btn", cx.entity_id()))
                                                    .small()
                                                    .label(SharedString::from("Next Week"))
                                                    .on_click(cx.action(ScheduleAction::SetDatePreset("next_week".to_string())))
                                            )
                                    )
                                    .child(Space::new(px(12.)))
                                    .child(
                                        Button::new(("custom-date-btn", cx.entity_id()))
                                            .outline()
                                            .w(px(120.))
                                            .label(SharedString::from("Custom Date"))
                                            .on_click(cx.action(ScheduleAction::OpenDatePicker))
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
                            .child(Divider::new())
                            // Recurrence Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        Label::new("Repeat")
                                            .font_size(px(14.))
                                            .font_weight(500)
                                            .mb(px(8.))
                                    )
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
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("daily".to_string())))
                                                    )
                                                    .child(
                                                        Button::new(("weekdays-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekdays"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("weekdays".to_string())))
                                                    )
                                                    .child(
                                                        Button::new(("weekends-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekends"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("weekends".to_string())))
                                                    )
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .child(
                                                        Button::new(("weekly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Weekly"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("weekly".to_string())))
                                                    )
                                                    .child(
                                                        Button::new(("monthly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Monthly"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("monthly".to_string())))
                                                    )
                                                    .child(
                                                        Button::new(("yearly-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Yearly"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("yearly".to_string())))
                                                    )
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(6.))
                                                    .child(
                                                        Button::new(("none-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("None"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("none".to_string())))
                                                    )
                                                    .child(
                                                        Button::new(("custom-recurrence-btn", cx.entity_id()))
                                                            .small()
                                                            .label(SharedString::from("Custom"))
                                                            .on_click(cx.action(ScheduleAction::SetRecurrence("custom".to_string())))
                                                    )
                                            )
                                    )
                                    .when(self.show_custom_recurrence, |this| {
                                        this.child(Space::new(px(12.)))
                                            .child(
                                                v_flex()
                                                    .p(px(12.))
                                                    .bg(cx.theme().palette().background.surface_1)
                                                    .border_radius(px(8.))
                                                    .child(
                                                        Label::new("Custom Repeat")
                                                            .font_size(px(12.))
                                                            .font_weight(500)
                                                            .mb(px(8.))
                                                    )
                                                    .child(
                                                        h_flex()
                                                            .align_items_center()
                                                            .child(
                                                                Label::new("Repeat every")
                                                                    .font_size(px(12.))
                                                                    .mr(px(8.))
                                                            )
                                                            .child(
                                                                NumberInput::new(("interval-input", cx.entity_id()))
                                                                    .value(self.custom_recurrence_interval)
                                                                    .min(1)
                                                                    .max(999)
                                                                    .w(px(80.))
                                                                    .on_change(cx.action(|value| ScheduleAction::SetCustomRecurrenceInterval(value)))
                                                            )
                                                            .child(
                                                                h_flex()
                                                                    .gap(px(4.))
                                                                    .child(
                                                                        Button::new(("minutes-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("min"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("minutes".to_string())))
                                                                            .active(self.custom_recurrence_type == "minutes")
                                                                    )
                                                                    .child(
                                                                        Button::new(("hours-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("hr"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("hours".to_string())))
                                                                            .active(self.custom_recurrence_type == "hours")
                                                                    )
                                                                    .child(
                                                                        Button::new(("days-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("day"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("days".to_string())))
                                                                            .active(self.custom_recurrence_type == "days")
                                                                    )
                                                                    .child(
                                                                        Button::new(("weeks-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("wk"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("weeks".to_string())))
                                                                            .active(self.custom_recurrence_type == "weeks")
                                                                    )
                                                                    .child(
                                                                        Button::new(("months-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("mo"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("months".to_string())))
                                                                            .active(self.custom_recurrence_type == "months")
                                                                    )
                                                                    .child(
                                                                        Button::new(("years-btn", cx.entity_id()))
                                                                            .tiny()
                                                                            .label(SharedString::from("yr"))
                                                                            .on_click(cx.action(ScheduleAction::SetCustomRecurrenceType("years".to_string())))
                                                                            .active(self.custom_recurrence_type == "years")
                                                                    )
                                                            )
                                                    )
                                                    .child(Space::new(px(12.)))
                                                    .child(
                                                        Label::new("End Repeat")
                                                            .font_size(px(12.))
                                                            .font_weight(500)
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
                                                                    .active(self.end_type == "never")
                                                            )
                                                            .child(
                                                                Button::new(("on-date-btn", cx.entity_id()))
                                                                    .small()
                                                                    .label(SharedString::from("On Date"))
                                                                    .on_click(cx.action(ScheduleAction::SetEndType("on_date".to_string())))
                                                                    .active(self.end_type == "on_date")
                                                            )
                                                            .child(
                                                                Button::new(("after-btn", cx.entity_id()))
                                                                    .small()
                                                                    .label(SharedString::from("After"))
                                                                    .on_click(cx.action(ScheduleAction::SetEndType("after".to_string())))
                                                                    .active(self.end_type == "after")
                                                            )
                                                    )
                                                    .when(self.end_type == "on_date", |this| {
                                                        this.child(Space::new(px(8.)))
                                                            .child(
                                                                DatePicker::new(&date_picker)
                                                                    .cleanable(true)
                                                                    .w(px(200.))
                                                                    .on_change(cx.action(|date| {
                                                                        ScheduleAction::SetEndDate(date.format("%Y-%m-%d").to_string())
                                                                    }))
                                                            )
                                                    })
                                                    .when(self.end_type == "after", |this| {
                                                        this.child(Space::new(px(8.)))
                                                            .child(
                                                                h_flex()
                                                                    .align_items_center()
                                                                    .child(
                                                                        NumberInput::new(("end-count-input", cx.entity_id()))
                                                                            .value(self.due_date.recurrency_count.max(1))
                                                                            .min(1)
                                                                            .max(999)
                                                                            .w(px(80.))
                                                                            .on_change(cx.action(|value| ScheduleAction::SetEndCount(value)))
                                                                    )
                                                                    .child(
                                                                        Label::new("occurrences")
                                                                            .font_size(px(12.))
                                                                            .ml(px(8.))
                                                                    )
                                                            )
                                                    })
                                            )
                                    )
                            )
                            .child(divider::Divider::new())
                            // Time Section
                            .child(
                                v_flex()
                                    .p(px(16.))
                                    .child(
                                        h_flex()
                                            .align_items_center()
                                            .justify_content_between()
                                            .child(
                                                Label::new("Time")
                                                    .font_size(px(14.))
                                                    .font_weight(500)
                                            )
                                            .child(
                                                Button::new(("time-toggle", cx.entity_id()))
                                                    .small()
                                                    .icon(IconName::Plus)
                                                    .on_click(cx.action(ScheduleAction::ToggleTimeInput))
                                            )
                                    )
                                    .when(self.show_time_input, |this| {
                                        this.child(Space::new(px(8.)))
                                            .child(
                                                TextInput::new(("time-input", cx.entity_id()))
                                                    .placeholder(SharedString::from("HH:MM"))
                                                    .value(SharedString::from(self.get_current_time()))
                                                    .w(px(100.))
                                                    .on_change(cx.action(|value| ScheduleAction::SetTime(value.to_string())))
                                            )
                                    })
                                    .otherwise(|this| {
                                        this.child(
                                            Label::new(&self.get_current_time())
                                                .font_size(px(12.))
                                                .text_color(cx.theme().palette().foreground.muted)
                                                .mt(px(4.))
                                        )
                                    })
                            )
                            .child(divider::Divider::new())
                            // Footer Buttons
                            .child(
                                h_flex()
                                    .p(px(16.))
                                    .justify_content_end()
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

// Create the complex button
create_complex_button!(ScheduleButton, ScheduleButtonState, ScheduleButtonEvent, "item-schedule");
