use chrono::Local;
use gpui::{
    Action, AppContext, Context, Corner, Entity, FocusHandle, InteractiveElement, ParentElement,
    Render, SharedString, Styled, Subscription, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Side,
    button::Button,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    menu::{DropdownMenu, PopupMenu},
    v_flex,
};
use serde::Deserialize;
use todos::{DueDate, enums::RecurrencyType};

use crate::{create_complex_button, impl_button_state_base};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct ScheduleAction(String);

pub enum ScheduleButtonEvent {
    DateSelected(String),
    TimeSelected(String),
    RecurrencySelected(RecurrencyType),
    Cleared,
}

pub struct ScheduleButtonState {
    focus_handle: FocusHandle,
    pub due_date: DueDate,
    selected_time: Option<String>,
    date_picker_state: Entity<DatePickerState>,
    show_date_picker: bool,
    _subscriptions: Vec<Subscription>,
}

impl_button_state_base!(ScheduleButtonState, ScheduleButtonEvent);

impl ScheduleButtonState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_event)];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            selected_time: None,
            date_picker_state,
            show_date_picker: false,
            _subscriptions,
        }
    }

    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        // 检查是否有实际变化
        let old_due_date = self.due_date.clone();
        let has_changed = old_due_date != due_date;

        self.due_date = due_date;
        self.sync_selected_time_from_due_date();
        if let Some(dt) = self.due_date.datetime() {
            let date = dt.date();
            self.date_picker_state.update(cx, |picker, cx| {
                picker.set_date(date, window, cx);
            });
        }

        // 只有在有实际变化时才通知UI刷新
        if has_changed {
            cx.notify();
        }
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
            let time_str = self.resolve_time_str();
            self.due_date.date = format!("{} {}:00", date_str, time_str);
            self.show_date_picker = false;
            cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
            cx.notify();
        }
    }

    fn on_select_action(
        &mut self,
        action: &ScheduleAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let today = Local::now().naive_local().date();
        let time_str = self.resolve_time_str();

        match action.0.as_str() {
            "today" => {
                let date_str = today.format("%Y-%m-%d").to_string();
                let new_date = format!("{} {}:00", date_str, time_str);
                if self.due_date.date != new_date {
                    self.due_date.date = new_date;
                    self.date_picker_state.update(cx, |picker, cx| {
                        picker.set_date(today, window, cx);
                    });
                    self.show_date_picker = false;
                    cx.emit(ScheduleButtonEvent::DateSelected("Today".to_string()));
                    cx.notify();
                }
            },
            "tomorrow" => {
                let tomorrow = today.succ_opt().unwrap_or(today);
                let date_str = tomorrow.format("%Y-%m-%d").to_string();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                self.date_picker_state.update(cx, |picker, cx| {
                    picker.set_date(tomorrow, window, cx);
                });
                self.show_date_picker = false;
                cx.emit(ScheduleButtonEvent::DateSelected("Tomorrow".to_string()));
            },
            "next_week" => {
                let next_week = today + chrono::Duration::days(7);
                let date_str = next_week.format("%Y-%m-%d").to_string();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                self.date_picker_state.update(cx, |picker, cx| {
                    picker.set_date(next_week, window, cx);
                });
                self.show_date_picker = false;
                cx.emit(ScheduleButtonEvent::DateSelected("Next week".to_string()));
            },
            "choose_date" => {
                self.show_date_picker = true;
                if self.due_date.date.is_empty() {
                    self.date_picker_state.update(cx, |picker, cx| {
                        picker.set_date(today, window, cx);
                    });
                }
            },
            "daily" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryDay;
                self.due_date.recurrency_interval = 1;
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryDay));
            },
            "weekdays" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                self.due_date.recurrency_weeks = "1,2,3,4,5".to_string();
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryWeek));
            },
            "weekends" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                self.due_date.recurrency_weeks = "0,6".to_string();
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryWeek));
            },
            "weekly" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryWeek));
            },
            "monthly" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryMonth;
                self.due_date.recurrency_interval = 1;
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryMonth));
            },
            "yearly" => {
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_type = RecurrencyType::EveryYear;
                self.due_date.recurrency_interval = 1;
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::EveryYear));
            },
            "none" => {
                self.due_date.is_recurring = false;
                self.due_date.recurrency_supported = false;
                self.due_date.recurrency_type = RecurrencyType::NONE;
                self.due_date.recurrency_interval = 0;
                cx.emit(ScheduleButtonEvent::RecurrencySelected(RecurrencyType::NONE));
            },
            s if s.starts_with("time_") => {
                let time = s.strip_prefix("time_").unwrap_or("00:00");
                self.apply_time_to_due_date(time);
                cx.emit(ScheduleButtonEvent::TimeSelected(time.to_string()));
            },
            "clear" => {
                if !self.due_date.date.is_empty() {
                    self.due_date = DueDate::default();
                    self.selected_time = None;
                    self.show_date_picker = false;
                    cx.emit(ScheduleButtonEvent::Cleared);
                    cx.notify();
                }
            },
            "done" => {
                self.show_date_picker = false;
                cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
            },
            _ => {},
        }
        cx.notify();
    }

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

    #[allow(dead_code)]
    fn get_repeat_text(&self) -> String {
        if !self.due_date.is_recurring {
            "None".to_string()
        } else {
            match self.due_date.recurrency_type {
                RecurrencyType::EveryDay => "Daily".to_string(),
                RecurrencyType::EveryWeek => {
                    if self.due_date.recurrency_weeks == "1,2,3,4,5" {
                        "Weekdays".to_string()
                    } else if self.due_date.recurrency_weeks == "0,6" {
                        "Weekends".to_string()
                    } else {
                        "Weekly".to_string()
                    }
                },
                RecurrencyType::EveryMonth => "Monthly".to_string(),
                RecurrencyType::EveryYear => "Yearly".to_string(),
                _ => "None".to_string(),
            }
        }
    }

    #[allow(dead_code)]
    fn get_time_text(&self) -> String {
        self.resolve_time_str()
    }

    fn apply_time_to_due_date(&mut self, time: &str) {
        self.selected_time = Some(time.to_string());
        if let Some((date_part, _)) = self.due_date.date.split_once(' ') {
            self.due_date.date = format!("{} {}:00", date_part, time);
        }
    }

    fn resolve_time_str(&self) -> String {
        self.selected_time
            .clone()
            .or_else(|| self.due_date.datetime().map(|dt| dt.time().format("%H:%M").to_string()))
            .unwrap_or_else(|| "17:30".to_string())
    }

    fn sync_selected_time_from_due_date(&mut self) {
        self.selected_time =
            self.due_date.datetime().map(|dt| dt.time().format("%H:%M").to_string());
    }

    fn get_choose_date_label(&self) -> String {
        if let Some(dt) = self.due_date.datetime() {
            format!("📅 Choose a date: {}", dt.date().format("%b %e"))
        } else {
            "📅 Choose a date".to_string()
        }
    }
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let choose_date_label = self.get_choose_date_label();
        let repeat_hint = self.get_repeat_text();
        let time_hint = self.get_time_text();
        let date_picker = self.date_picker_state.clone();
        let date_picker_for_when = date_picker.clone();

        v_flex()
            .on_action(cx.listener(Self::on_select_action))
            .child(
                Button::new(("item-schedule", cx.entity_id()))
                    .outline()
                    .tooltip("set schedule")
                    .icon(IconName::Calendar)
                    .label(SharedString::from(self.get_display_text()))
                    .dropdown_menu_with_anchor(Corner::TopLeft, move |this, window, cx| {
                        let choose_date_label = choose_date_label.clone();
                        let repeat_hint = repeat_hint.clone();
                        let time_hint = time_hint.clone();

                        this.check_side(Side::Left)
                            .min_w(px(260.))
                            .menu("★ Today", Box::new(ScheduleAction("today".to_string())))
                            .menu("☐ Tomorrow", Box::new(ScheduleAction("tomorrow".to_string())))
                            .menu("↷ Next week", Box::new(ScheduleAction("next_week".to_string())))
                            .separator()
                            .menu(
                                choose_date_label,
                                Box::new(ScheduleAction("choose_date".to_string())),
                            )
                            .separator()
                            .submenu(
                                format!("⟳ Repeat   {}", repeat_hint),
                                window,
                                cx,
                                |this: PopupMenu, _window, _cx| {
                                    this.menu(
                                        "Every day",
                                        Box::new(ScheduleAction("daily".to_string())),
                                    )
                                    .menu(
                                        "Weekdays",
                                        Box::new(ScheduleAction("weekdays".to_string())),
                                    )
                                    .menu(
                                        "Weekends",
                                        Box::new(ScheduleAction("weekends".to_string())),
                                    )
                                    .menu("Weekly", Box::new(ScheduleAction("weekly".to_string())))
                                    .menu(
                                        "Monthly",
                                        Box::new(ScheduleAction("monthly".to_string())),
                                    )
                                    .menu("Yearly", Box::new(ScheduleAction("yearly".to_string())))
                                    .menu("None", Box::new(ScheduleAction("none".to_string())))
                                },
                            )
                            .separator()
                            .submenu(
                                format!("⏰ Time   {}", time_hint),
                                window,
                                cx,
                                |this: PopupMenu, _window, _cx| {
                                    this.menu(
                                        "17:30",
                                        Box::new(ScheduleAction("time_17:30".to_string())),
                                    )
                                    .menu(
                                        "09:00",
                                        Box::new(ScheduleAction("time_09:00".to_string())),
                                    )
                                    .menu(
                                        "12:00",
                                        Box::new(ScheduleAction("time_12:00".to_string())),
                                    )
                                    .menu(
                                        "14:00",
                                        Box::new(ScheduleAction("time_14:00".to_string())),
                                    )
                                    .menu(
                                        "18:00",
                                        Box::new(ScheduleAction("time_18:00".to_string())),
                                    )
                                    .menu(
                                        "20:00",
                                        Box::new(ScheduleAction("time_20:00".to_string())),
                                    )
                                },
                            )
                    }),
            )
            .when(self.show_date_picker, move |this| {
                this.child(DatePicker::new(&date_picker_for_when).cleanable(true).w(px(260.)))
            })
    }
}

create_complex_button!(ScheduleButton, ScheduleButtonState, ScheduleButtonEvent, "item-schedule");
