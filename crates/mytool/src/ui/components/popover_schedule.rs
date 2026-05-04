use chrono::Local;
use gpui::{
    Action, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::InputState,
    menu::DropdownMenu,
    popover::Popover,
    radio::{Radio, RadioGroup},
    v_flex,
};
use serde::Deserialize;
use todos::DueDate;

use crate::{create_button_wrapper, impl_button_state_base};

#[derive(Clone, PartialEq, Deserialize)]
pub enum ScheduleButtonEvent {
    DateSelected(String),
    TimeSelected(String),
    Cleared,
}

#[derive(Clone)]
struct TimeSelected(String);

impl Action for TimeSelected {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }

    fn partial_eq(&self, _other: &dyn Action) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "TimeSelected"
    }

    fn name_for_type() -> &'static str {
        "TimeSelected"
    }

    fn build(_: serde_json::Value) -> Result<Box<dyn Action>, anyhow::Error> {
        Err(anyhow::anyhow!("Cannot build TimeSelected from JSON"))
    }
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum SchedulePreset {
    Today,
    Tomorrow,
    NextWeek,
    Custom,
}

impl SchedulePreset {
    pub fn to_label(&self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Tomorrow => "Tomorrow",
            Self::NextWeek => "Next week",
            Self::Custom => "Choose a date...",
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
    time_input: Entity<InputState>,
    selected_time: String,
    _subscriptions: Vec<gpui::Subscription>,
}

const TIME_OPTIONS: [&str; 5] = ["9:00", "12:00", "14:00", "17:00", "20:00"];

impl ScheduleForm {
    pub fn new(
        parent: Entity<ScheduleButtonState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let time_input = cx.new(|cx| InputState::new(window, cx).placeholder("17:00"));
        time_input.update(cx, |input, cx| {
            input.set_value("17:00", window, cx);
        });
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_event)];

        Self {
            parent,
            selected_preset_index: 0,
            date_picker_state,
            custom_date: None,
            time_input,
            selected_time: "17:00".to_string(),
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

            self.selected_time = time_str.clone();
            self.time_input.update(cx, |input, cx| {
                input.set_value(&time_str, window, cx);
            });
        } else {
            self.selected_preset_index = 0;
            self.custom_date = None;
            self.selected_time = "17:00".to_string();
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
        if let Some(date_str) = date.format("%Y-%m-%d") {
            if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                self.custom_date = Some(parsed_date);
            }
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

    fn resolve_time_str(&mut self, cx: &mut Context<Self>) -> String {
        self.time_input.update(cx, |input, _| input.value().clone()).to_string()
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
        let selected_time = self.selected_time.clone();

        let radio_group =
            RadioGroup::vertical("schedule-preset-group")
                .selected_index(Some(selected_index))
                .on_click(cx.listener(|this, index, _, cx| {
                    this.selected_preset_index = *index;
                    cx.notify();
                }))
                .children(presets.iter().map(|preset| {
                    Radio::new(format!("preset-{:?}", preset)).label(preset.to_label())
                }));

        let time_dropdown = Button::new("time-dropdown")
            .small()
            .outline()
            .label(selected_time)
            .tooltip("Select time")
            .dropdown_menu_with_anchor(
                gpui::Anchor::TopLeft,
                move |menu: gpui_component::menu::PopupMenu, _, _| {
                    let mut menu = menu.scrollable(true).max_h(px(200.)).min_w(px(100.));
                    for time in TIME_OPTIONS {
                        menu = menu.menu(
                            SharedString::from(time),
                            Box::new(TimeSelected(time.to_string())),
                        );
                    }
                    menu
                },
            );

        v_flex()
            .gap_3()
            .p_3()
            .w(px(280.))
            .on_action(cx.listener(Self::on_time_selected))
            .child(radio_group)
            .when(is_custom, move |this| {
                this.child(DatePicker::new(&date_picker).cleanable(true).w(px(240.)))
            })
            .child(div().h_1().bg(gpui::rgb(0xe0e0e0)).mx_3())
            .child(v_flex().gap_2().child("Time").child(time_dropdown))
            .child(div().h_1().bg(gpui::rgb(0xe0e0e0)).mx_3())
            .child(Button::new("apply-btn").w_full().primary().label("Apply").on_click(
                cx.listener(move |this, _, _window, cx| {
                    if is_custom {
                        this.apply_custom_date(cx);
                    } else {
                        let preset = this.get_selected_preset();
                        this.apply_date_preset(preset, cx);
                    }
                    cx.emit(DismissEvent);
                }),
            ))
    }
}

impl ScheduleForm {
    fn on_time_selected(
        &mut self,
        action: &TimeSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_time = action.0.clone();
        self.time_input.update(cx, |input, cx| {
            input.set_value(&action.0, window, cx);
        });
        cx.notify();
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
        cx.notify();
    }

    fn get_display_text(&self) -> String {
        if self.due_date.date.is_empty() {
            "Schedule".to_string()
        } else {
            let today = Local::now().naive_local().date();
            if let Some(dt) = self.due_date.datetime() {
                let date = dt.date();
                let time = dt.time();
                let time_str = time.format("%H:%M").to_string();

                if date == today {
                    format!("Today at {}", time_str)
                } else if date == today.succ_opt().unwrap_or(today) {
                    format!("Tomorrow at {}", time_str)
                } else {
                    format!("{} at {}", date.format("%b %d"), time_str)
                }
            } else {
                "Schedule".to_string()
            }
        }
    }
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let display_text = self.get_display_text();
        let form = self.form.clone();

        v_flex().track_focus(&self.focus_handle).child(
            Popover::new("schedule-popover")
                .p_0()
                .text_sm()
                .open(self.popover_open)
                .on_open_change(cx.listener(|this, open, _, cx| {
                    this.popover_open = *open;
                    cx.notify();
                }))
                .trigger(
                    Button::new(("item-schedule", cx.entity_id()))
                        .outline()
                        .tooltip("set schedule")
                        .icon(IconName::Calendar)
                        .label(SharedString::from(display_text)),
                )
                .child(form.clone()),
        )
    }
}

create_button_wrapper!(ScheduleButton, ScheduleButtonState, "item-schedule");
