use chrono::Local;
use gpui::{
    Action, App, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div,
};
use gpui_component::{
    IconName, Side, Sizable, Size, StyleSized, StyledExt as _,
    button::Button,
    menu::{DropdownMenu, PopupMenu},
    v_flex,
};
use serde::Deserialize;
use todos::{enums::RecurrencyType, objects::DueDate};

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
}

impl EventEmitter<ScheduleButtonEvent> for ScheduleButtonState {}

impl Focusable for ScheduleButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ScheduleButtonState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), due_date: DueDate::default(), selected_time: None }
    }

    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    pub fn set_due_date(&mut self, due_date: DueDate, _: &mut Window, cx: &mut Context<Self>) {
        self.due_date = due_date;
        cx.notify()
    }

    fn on_select_action(
        &mut self,
        action: &ScheduleAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let today = Local::now().naive_local().date();
        let time_str = self.selected_time.clone().unwrap_or_else(|| "00:00".to_string());

        match action.0.as_str() {
            "today" => {
                let date_str = today.format("%Y-%m-%d").to_string();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                cx.emit(ScheduleButtonEvent::DateSelected("Today".to_string()));
            },
            "tomorrow" => {
                let tomorrow = today.succ_opt().unwrap_or(today);
                let date_str = tomorrow.format("%Y-%m-%d").to_string();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                cx.emit(ScheduleButtonEvent::DateSelected("Tomorrow".to_string()));
            },
            "next_week" => {
                let next_week = today + chrono::Duration::days(7);
                let date_str = next_week.format("%Y-%m-%d").to_string();
                self.due_date.date = format!("{} {}:00", date_str, time_str);
                cx.emit(ScheduleButtonEvent::DateSelected("Next week".to_string()));
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
                self.selected_time = Some(time.to_string());
                cx.emit(ScheduleButtonEvent::TimeSelected(time.to_string()));
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

    fn get_time_text(&self) -> String {
        self.selected_time.clone().unwrap_or_else(|| "00:00".to_string())
    }
}

/// A ScheduleButton element.
#[derive(IntoElement)]
pub struct ScheduleButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ScheduleButtonState>,
}

impl Sizable for ScheduleButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ScheduleButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ScheduleButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        v_flex().on_action(cx.listener(Self::on_select_action)).child(
            Button::new(("item-schedule", cx.entity_id()))
                .outline()
                .tooltip("set schedule")
                .icon(IconName::Calendar)
                .label(SharedString::from(self.get_display_text()))
                .dropdown_menu_with_anchor(Corner::TopLeft, move |this, window, cx| {
                    this.check_side(Side::Left)
                        .label("Date")
                        .menu("Today", Box::new(ScheduleAction("today".to_string())))
                        .menu("Tomorrow", Box::new(ScheduleAction("tomorrow".to_string())))
                        .menu("Next week", Box::new(ScheduleAction("next_week".to_string())))
                        .separator()
                        .submenu("Repeat", window, cx, |this: PopupMenu, _window, _cx| {
                            this.menu("Daily", Box::new(ScheduleAction("daily".to_string())))
                                .menu("Weekdays", Box::new(ScheduleAction("weekdays".to_string())))
                                .menu("Weekends", Box::new(ScheduleAction("weekends".to_string())))
                                .menu("Weekly", Box::new(ScheduleAction("weekly".to_string())))
                                .menu("Monthly", Box::new(ScheduleAction("monthly".to_string())))
                                .menu("Yearly", Box::new(ScheduleAction("yearly".to_string())))
                                .menu("None", Box::new(ScheduleAction("none".to_string())))
                        })
                        .separator()
                        .submenu("Time", window, cx, |this: PopupMenu, _window, _cx| {
                            this.menu("09:00", Box::new(ScheduleAction("time_09:00".to_string())))
                                .menu("12:00", Box::new(ScheduleAction("time_12:00".to_string())))
                                .menu("14:00", Box::new(ScheduleAction("time_14:00".to_string())))
                                .menu("18:00", Box::new(ScheduleAction("time_18:00".to_string())))
                                .menu("20:00", Box::new(ScheduleAction("time_20:00".to_string())))
                        })
                }),
        )
    }
}

impl ScheduleButton {
    /// Create a new ScheduleButton with the given [`ScheduleButtonState`].
    pub fn new(state: &Entity<ScheduleButtonState>) -> Self {
        Self {
            id: ("item-schedule", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ScheduleButton {
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
