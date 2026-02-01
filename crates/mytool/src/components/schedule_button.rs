use chrono::Local;
use gpui::{
    Action, App, AppContext, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce,
    SharedString, StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    menu::DropdownMenu,
    v_flex,
};
use serde::Deserialize;
use todos::{enums::RecurrencyType, objects::DueDate};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct SetTimeAction(String);

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct SetRecurrencyAction(RecurrencyType);

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct ToggleCustomRecurrencyAction;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct ClearAction;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = schedule_button, no_json)]
struct DoneAction;

pub enum ScheduleButtonEvent {
    DateSelected(String),
    TimeSelected(String),
    RecurrencySelected(RecurrencyType),
    Cleared,
    Done(DueDate),
}

pub struct ScheduleButtonState {
    focus_handle: FocusHandle,
    pub due_date: DueDate,
    selected_time: Option<String>,
    date_picker_state: Entity<DatePickerState>,
    show_popover: bool,
    show_custom_recurrency: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ScheduleButtonEvent> for ScheduleButtonState {}

impl Focusable for ScheduleButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ScheduleButtonState {
    fn on_action(&mut self, action: &dyn Action, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(action) = action.as_any().downcast_ref::<SetTimeAction>() {
            self.set_time(&action.0, cx);
        } else if let Some(action) = action.as_any().downcast_ref::<SetRecurrencyAction>() {
            self.set_recurrency(action.0.clone(), cx);
        } else if let Some(_) = action.as_any().downcast_ref::<ToggleCustomRecurrencyAction>() {
            self.show_custom_recurrency = !self.show_custom_recurrency;
            cx.notify();
        } else if let Some(_) = action.as_any().downcast_ref::<ClearAction>() {
            self.clear(cx);
        } else if let Some(_) = action.as_any().downcast_ref::<DoneAction>() {
            self.done(cx);
        }
    }
}

impl ScheduleButtonState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let _subscriptions = vec![cx.subscribe_in(&date_picker_state, window, Self::on_date_event)];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            selected_time: None,
            date_picker_state,
            show_popover: false,
            show_custom_recurrency: false,
            _subscriptions,
        }
    }

    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        self.due_date = due_date;
        self.sync_selected_time_from_due_date();
        if let Some(dt) = self.due_date.datetime() {
            let date = dt.date();
            self.date_picker_state.update(cx, |picker, cx| {
                picker.set_date(date, window, cx);
            });
        }
        cx.notify()
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
            cx.emit(ScheduleButtonEvent::DateSelected(self.get_display_text()));
            cx.notify();
        }
    }

    fn set_time(&mut self, time: &str, cx: &mut Context<Self>) {
        self.selected_time = Some(time.to_string());
        if let Some((date_part, _)) = self.due_date.date.split_once(' ') {
            self.due_date.date = format!("{} {}:00", date_part, time);
        } else {
            let today = Local::now().naive_local().date();
            let date_str = today.format("%Y-%m-%d").to_string();
            self.due_date.date = format!("{} {}:00", date_str, time);
        }
        cx.emit(ScheduleButtonEvent::TimeSelected(time.to_string()));
        cx.notify();
    }

    fn set_recurrency(&mut self, recurrency_type: RecurrencyType, cx: &mut Context<Self>) {
        self.due_date.is_recurring = true;
        self.due_date.recurrency_supported = true;
        self.due_date.recurrency_type = recurrency_type.clone();
        self.due_date.recurrency_interval = 1;
        self.show_custom_recurrency = false;
        cx.emit(ScheduleButtonEvent::RecurrencySelected(recurrency_type));
        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.due_date = DueDate::default();
        self.selected_time = None;
        self.show_custom_recurrency = false;
        cx.emit(ScheduleButtonEvent::Cleared);
        cx.notify();
    }

    fn done(&mut self, cx: &mut Context<Self>) {
        self.show_popover = false;
        cx.emit(ScheduleButtonEvent::Done(self.due_date.clone()));
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
                RecurrencyType::EveryWeek => "Weekly".to_string(),
                RecurrencyType::EveryMonth => "Monthly".to_string(),
                RecurrencyType::EveryYear => "Yearly".to_string(),
                _ => "None".to_string(),
            }
        }
    }

    fn get_time_text(&self) -> String {
        self.resolve_time_str()
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
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let date_picker = self.date_picker_state.clone();
        let entity = cx.entity();
        let show_popover = self.show_popover;
        let show_custom_recurrency = self.show_custom_recurrency;
        let time_text = self.get_time_text();
        let repeat_text = self.get_repeat_text();
        let background_color = cx.theme().background;
        let border_color = cx.theme().border;
        let entity_id = cx.entity_id();

        v_flex()
                .child(
                    Button::new(("item-schedule", entity_id))
                        .outline()
                        .tooltip("set schedule")
                        .icon(IconName::Calendar)
                        .label(SharedString::from(self.get_display_text()))
                        .on_click({
                            let entity = entity.clone();
                            move |_ev, _window, app| {
                                app.update_entity(&entity, |this, cx| {
                                    this.show_popover = !this.show_popover;
                                    cx.notify();
                                });
                            }
                        }),
                )
                .when(show_popover, move |this| {
                    this.child(
                        div()
                            .bg(background_color)
                            .border_1()
                            .border_color(border_color)
                            .rounded_lg()
                            .p_3()
                            .gap_3()
                            .flex_col()
                            .w(px(320.))
                            // Part 1: Date and Time Selection
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                DatePicker::new(&date_picker)
                                                    .small()
                                                    .w(px(180.)),
                                            )
                                            .child(
                                                Button::new(("time-dropdown", entity_id))
                                                    .small()
                                                    .label(format!("⏰ {}", time_text))
                                                    .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                                        move |menu, _window, _cx| {
                                                            menu.menu("09:00", Box::new(SetTimeAction("09:00".to_string())))
                                                                .menu("12:00", Box::new(SetTimeAction("12:00".to_string())))
                                                                .menu("14:00", Box::new(SetTimeAction("14:00".to_string())))
                                                                .menu("17:30", Box::new(SetTimeAction("17:30".to_string())))
                                                                .menu("18:00", Box::new(SetTimeAction("18:00".to_string())))
                                                                .menu("20:00", Box::new(SetTimeAction("20:00".to_string())))
                                                        }
                                                    }),
                                            ),
                                    )
                            )
                            // Part 2: Recurrency Selection
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Button::new(("repeat-dropdown", entity_id))
                                            .small()
                                            .label(format!("⟳ Repeat: {}", repeat_text))
                                            .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                                move |menu, _window, _cx| {
                                                    menu.menu("Daily", Box::new(SetRecurrencyAction(RecurrencyType::EveryDay)))
                                                        .menu("Weekly", Box::new(SetRecurrencyAction(RecurrencyType::EveryWeek)))
                                                        .menu("Monthly", Box::new(SetRecurrencyAction(RecurrencyType::EveryMonth)))
                                                        .menu("Yearly", Box::new(SetRecurrencyAction(RecurrencyType::EveryYear)))
                                                        .menu("Custom Date", Box::new(ToggleCustomRecurrencyAction))
                                                }
                                            }),
                                    ),
                            )
                            .when(show_custom_recurrency, move |this: gpui::Div| {
                                this.child(
                                    div()
                                        .border_t_1()
                                        .border_color(border_color)
                                        .pt_2()
                                        .child(
                                            DatePicker::new(&date_picker)
                                                .small()
                                                .w(px(300.)),
                                        ),
                                )
                            })
                            // Bottom buttons
                            .child(
                                h_flex()
                                    .gap_2()
                                    .justify_between()
                                    .child(
                                        Button::new(("clear-btn", entity_id))
                                            .label("Clear")
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, _window, app: &mut App| {
                                                    app.update_entity(&entity, |this: &mut ScheduleButtonState, cx: &mut Context<ScheduleButtonState>| {
                                                        this.clear(cx);
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new(("done-btn", entity_id))
                                            .primary()
                                            .label("Done")
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, _window, app: &mut App| {
                                                    app.update_entity(&entity, |this: &mut ScheduleButtonState, cx: &mut Context<ScheduleButtonState>| {
                                                        this.done(cx);
                                                    });
                                                }
                                            }),
                                    ),
                            ),
                    )
                })
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
