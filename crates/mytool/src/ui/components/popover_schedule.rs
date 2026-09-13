use chrono::{Datelike, Local};
use gpui::{
    App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    IndexPath, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    form::{field, v_form},
    popover::Popover,
    radio::{Radio, RadioGroup},
    select::{Select, SelectState},
    separator::Separator,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use serde::Deserialize;
use todos::DueDate;

use crate::{create_button_wrapper, impl_button_state_base};

#[derive(Clone, PartialEq, Deserialize)]
pub enum ScheduleButtonEvent {
    DateSelected(String),
    TimeSelected(String),
    Cleared,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum SchedulePreset {
    Today,
    Tomorrow,
    NextWeek,
    Custom,
}

impl SchedulePreset {
    pub fn to_label(self) -> String {
        match self {
            Self::Today => t!("todo.due.today").to_string(),
            Self::Tomorrow => t!("todo.due.tomorrow").to_string(),
            Self::NextWeek => t!("todo.due.next_week").to_string(),
            Self::Custom => t!("todo.due.pick").to_string(),
        }
    }

    pub fn all_presets() -> Vec<Self> {
        vec![Self::Today, Self::Tomorrow, Self::NextWeek, Self::Custom]
    }
}

pub struct ScheduleForm {
    parent: Entity<ScheduleButtonState>,
    selected_preset_index: usize,
    date_picker_state: Entity<DatePickerState>,
    custom_date: Option<chrono::NaiveDate>,
    time_select: Entity<SelectState<Vec<SharedString>>>,
    _subscriptions: Vec<gpui::Subscription>,
}

const TIME_OPTIONS: [&str; 5] = ["9:00", "12:00", "14:00", "17:00", "20:00"];

fn time_items(extra: Option<&str>) -> Vec<SharedString> {
    let mut items: Vec<SharedString> =
        TIME_OPTIONS.iter().map(|t| SharedString::from(*t)).collect();
    if let Some(time) = extra
        && !items.iter().any(|item| item.as_ref() == time)
    {
        items.push(SharedString::from(time.to_string()));
    }
    items
}

impl ScheduleForm {
    pub fn new(
        parent: Entity<ScheduleButtonState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let time_index = TIME_OPTIONS.iter().position(|t| *t == "17:00").unwrap_or(3);
        let time_select = cx.new(|cx| {
            SelectState::new(
                time_items(None),
                Some(IndexPath::default().row(time_index)),
                window,
                cx,
            )
        });
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_event)];

        Self {
            parent,
            selected_preset_index: 0,
            date_picker_state,
            custom_date: None,
            time_select,
            _subscriptions,
        }
    }

    pub fn sync_from_parent(
        &mut self,
        due_date: &DueDate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let today = Local::now().naive_local().date();

        if let Some(dt) = due_date.datetime() {
            let date = dt.date();
            let time_str = dt.time().format("%H:%M").to_string();

            if date == today {
                self.selected_preset_index = 0;
            } else if date == today.succ_opt().unwrap_or(today) {
                self.selected_preset_index = 1;
            } else if date == today + chrono::Duration::days(7) {
                self.selected_preset_index = 2;
            } else {
                self.selected_preset_index = 3;
                self.custom_date = Some(date);
                self.date_picker_state.update(cx, |picker, cx| picker.set_date(date, window, cx));
            }

            self.set_time(&time_str, window, cx);
        } else {
            self.selected_preset_index = 0;
            self.custom_date = None;
            self.set_time("17:00", window, cx);
        }

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
        if let Some(date_str) = date.format("%Y-%m-%d")
            && let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        {
            self.custom_date = Some(parsed_date);
        }
        cx.notify();
    }

    fn apply_date_preset(&mut self, preset: SchedulePreset, cx: &mut Context<Self>) {
        let today = Local::now().naive_local().date();
        let time_str = self.resolve_time_str(cx);

        let date = match preset {
            SchedulePreset::Today => today,
            SchedulePreset::Tomorrow => today.succ_opt().unwrap_or(today),
            SchedulePreset::NextWeek => today + chrono::Duration::days(7),
            SchedulePreset::Custom => return,
        };

        let date_str = date.format("%Y-%m-%d").to_string();
        let new_date = format!("{} {}:00", date_str, time_str);

        self.parent.update(cx, |parent, _cx| parent.due_date.date = new_date);
    }

    fn apply_custom_date(&mut self, cx: &mut Context<Self>) {
        if let Some(date) = self.custom_date {
            let time_str = self.resolve_time_str(cx);
            let date_str = date.format("%Y-%m-%d").to_string();
            let new_date = format!("{} {}:00", date_str, time_str);

            self.parent.update(cx, |parent, _cx| parent.due_date.date = new_date);
        }
    }

    fn set_time(&mut self, time: &str, window: &mut Window, cx: &mut Context<Self>) {
        let value = SharedString::from(time.to_string());
        self.time_select.update(cx, |select, cx| {
            select.set_items(time_items(Some(time)), window, cx);
            select.set_selected_value(&value, window, cx);
        });
    }

    fn resolve_time_str(&self, cx: &App) -> String {
        self.time_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| SharedString::from("17:00"))
            .to_string()
    }

    fn get_selected_preset(&self) -> SchedulePreset {
        let presets = SchedulePreset::all_presets();
        presets.get(self.selected_preset_index).copied().unwrap_or(SchedulePreset::Today)
    }
}

impl Focusable for ScheduleForm {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.parent.read(cx).focus_handle(cx)
    }
}

impl EventEmitter<DismissEvent> for ScheduleForm {}

impl Render for ScheduleForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_custom = self.get_selected_preset() == SchedulePreset::Custom;
        let selected_index = self.selected_preset_index;
        let presets = SchedulePreset::all_presets();
        let date_picker = self.date_picker_state.clone();
        let time_select = self.time_select.clone();

        let has_date = !self.parent.read(cx).due_date.date.is_empty();
        let radio_group =
            RadioGroup::vertical("schedule-preset-group")
                .selected_index(Some(selected_index))
                .on_click(cx.listener(|this, index, _, cx| {
                    this.selected_preset_index = *index;
                    let preset = this.get_selected_preset();
                    if preset == SchedulePreset::Custom {
                        cx.notify();
                    } else {
                        this.apply_date_preset(preset, cx);
                        cx.emit(DismissEvent);
                    }
                }))
                .children(presets.iter().map(|preset| {
                    Radio::new(format!("preset-{:?}", preset)).label(preset.to_label())
                }));

        v_flex()
            .gap_2()
            .p_2()
            .w(px(240.))
            .child(radio_group)
            .when(is_custom, move |this| {
                this.child(DatePicker::new(&date_picker).cleanable(true).w_full())
            })
            .child(Separator::horizontal())
            .child(
                v_form().child(
                    field()
                        .label(t!("todo.due.time").to_string())
                        .child(Select::new(&time_select).small().placeholder("17:00")),
                ),
            )
            .child(Separator::horizontal())
            .child(
                Button::new("apply-btn")
                    .w_full()
                    .primary()
                    .label(t!("todo.confirm").to_string())
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        if this.get_selected_preset() == SchedulePreset::Custom {
                            this.apply_custom_date(cx);
                        } else {
                            let preset = this.get_selected_preset();
                            this.apply_date_preset(preset, cx);
                        }
                        cx.emit(DismissEvent);
                    })),
            )
            .when(has_date, |this| {
                this.child(
                    Button::new("clear-date")
                        .w_full()
                        .ghost()
                        .label(t!("todo.due.clear").to_string())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.parent.update(cx, |parent, cx| {
                                parent.due_date.date.clear();
                                cx.emit(ScheduleButtonEvent::Cleared);
                            });
                            cx.emit(DismissEvent);
                        })),
                )
            })
    }
}

pub struct ScheduleButtonState {
    focus_handle: FocusHandle,
    pub due_date: DueDate,
    form: Entity<ScheduleForm>,
    popover_open: bool,
    _subscriptions: Vec<gpui::Subscription>,
}

impl_button_state_base!(ScheduleButtonState, ScheduleButtonEvent);

impl ScheduleButtonState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let parent = cx.entity();
        let form = cx.new(|cx| ScheduleForm::new(parent, window, cx));
        let _subscriptions = vec![cx.subscribe_in(&form, window, Self::on_dismiss_event)];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            form,
            popover_open: false,
            _subscriptions,
        }
    }

    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        let old_due_date = self.due_date.clone();
        let has_changed = old_due_date != due_date;

        self.due_date = due_date.clone();

        self.form.update(cx, |form, cx| {
            form.sync_from_parent(&due_date, window, cx);
        });

        if has_changed {
            cx.notify();
        }
    }

    fn on_dismiss_event(
        &mut self,
        _state: &Entity<ScheduleForm>,
        _event: &DismissEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.popover_open = false;

        // 🔧 修复：当用户选择日期后关闭 popover 时，发射 DateSelected 事件
        // 这样 ItemInfoState 才能收到通知并更新 state_manager.item.due
        if !self.due_date.date.is_empty() {
            cx.emit(ScheduleButtonEvent::DateSelected(self.due_date.date.clone()));
        }

        cx.notify();
    }

    fn get_display_text(&self) -> String {
        if self.due_date.date.is_empty() {
            t!("todo.due.label").to_string()
        } else {
            let today = Local::now().naive_local().date();
            if let Some(dt) = self.due_date.datetime() {
                let date = dt.date();
                let time = dt.time();
                let time_str = time.format("%H:%M").to_string();

                if date == today {
                    t!("todo.due.today_time", time => time_str.as_str()).to_string()
                } else if date == today.succ_opt().unwrap_or(today) {
                    t!("todo.due.tomorrow_time", time => time_str.as_str()).to_string()
                } else {
                    t!("todo.due.md_time", month => date.month().to_string(), day => date.day().to_string(), time => time_str.as_str()).to_string()
                }
            } else {
                t!("todo.due.label").to_string()
            }
        }
    }
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let display_text = self.get_display_text();
        let form = self.form.clone();
        let has_date = !self.due_date.date.is_empty();
        let mut trigger = Button::new(("item-schedule", cx.entity_id()))
            .small()
            .ghost()
            .compact()
            .tooltip(t!("todo.due.set_tooltip").to_string())
            .icon(IconName::Calendar);
        if has_date {
            trigger = trigger.label(SharedString::from(display_text));
        }

        v_flex().track_focus(&self.focus_handle).child(
            Popover::new("schedule-popover")
                .p_0()
                .text_sm()
                .open(self.popover_open)
                .on_open_change(cx.listener(|this, open, _, cx| {
                    this.popover_open = *open;
                    cx.notify();
                }))
                .trigger(trigger)
                .child(form.clone()),
        )
    }
}

create_button_wrapper!(ScheduleButton, ScheduleButtonState, "item-schedule");
