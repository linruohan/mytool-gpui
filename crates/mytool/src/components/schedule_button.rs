use chrono::Local;
use gpui::{
    App, AppContext, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::{InputEvent, InputState, NumberInput},
    menu::DropdownMenu,
    v_flex,
};
use todos::{
    enums::{RecurrencyEndType, RecurrencyType},
    objects::DueDate,
};

const DEFAULT_TIME: &str = "17:30";
const TIME_OPTIONS: &[&str] = &["09:00", "12:00", "14:00", "17:30", "18:00", "20:00"];
const RECURRENCY_UNITS: &[&str] =
    &["Minute(s)", "Hour(s)", "Day(s)", "Week(s)", "Month(s)", "Year(s)"];

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

    // two independent date picker states to avoid shared-state conflicts
    date_picker_state: Entity<DatePickerState>,
    recurrency_date_picker_state: Entity<DatePickerState>,

    show_popover: bool,
    show_custom_recurrency: bool,

    recurrency_interval_input: Entity<InputState>,
    recurrency_interval: i64, // Store the value
    recurrency_unit: String,
    recurrency_end_type: String, // "Never", "OnDate", "After"
    recurrency_count_input: Entity<InputState>,
    recurrency_count: i64, // Store the value

    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ScheduleButtonEvent> for ScheduleButtonState {}

impl Focusable for ScheduleButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ScheduleButtonState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state =
            cx.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
        let recurrency_date_picker_state =
            cx.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
        let recurrency_interval_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let recurrency_count_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));

        // Set default date for date picker to today
        let today = Local::now().naive_local().date();
        date_picker_state.update(cx, |picker, ctx| {
            picker.set_date(today, window, ctx);
        });

        let _subscriptions = vec![
            cx.subscribe_in(&date_picker_state, window, Self::on_date_event),
            cx.subscribe_in(&recurrency_date_picker_state, window, Self::on_recurrency_date_event),
            cx.subscribe_in(&recurrency_interval_input, window, Self::on_recurrency_interval_event),
            cx.subscribe_in(&recurrency_count_input, window, Self::on_recurrency_count_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            due_date: DueDate::default(),
            selected_time: None,
            date_picker_state,
            recurrency_date_picker_state,
            show_popover: false,
            show_custom_recurrency: false,
            recurrency_interval_input,
            recurrency_interval: 1,
            recurrency_unit: RECURRENCY_UNITS[2].to_string(), // "Day(s)"
            recurrency_end_type: "Never".to_string(),
            recurrency_count_input,
            recurrency_count: 0,
            _subscriptions,
        }
    }

    pub fn from_due_date(due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = Self::new(window, cx);
        state.sync_from_due_date(due_date, window, cx);
        state
    }

    pub fn sync_from_due_date(
        &mut self,
        due_date: DueDate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.due_date = due_date.clone();
        self.sync_selected_time_from_due_date();
        self.recurrency_unit =
            Self::get_recurrency_unit_from_type(&due_date.recurrency_type).to_string();
        self.recurrency_end_type = Self::get_recurrency_end_type_string(&due_date).to_string();

        // Sync recurrency interval
        self.recurrency_interval =
            if due_date.recurrency_interval > 0 { due_date.recurrency_interval } else { 1 };

        // Sync recurrency count
        self.recurrency_count =
            if due_date.recurrency_count > 0 { due_date.recurrency_count } else { 0 };

        // Sync input fields with the values
        self.recurrency_interval_input.update(cx, |input, ctx| {
            input.set_value(self.recurrency_interval.to_string(), window, ctx);
        });
        self.recurrency_count_input.update(cx, |input, ctx| {
            input.set_value(self.recurrency_count.to_string(), window, ctx);
        });

        // Show custom recurrency panel if recurring
        if due_date.is_recurring {
            self.show_custom_recurrency = true;
        }

        // Sync recurrency end date picker if end type is OnDate
        if due_date.end_type() == RecurrencyEndType::OnDate
            && let Some(end_dt) = due_date.end_datetime() {
            let end_date = end_dt.date();
            self.recurrency_date_picker_state.update(cx, |picker, ctx| {
                picker.set_date(end_date, window, ctx);
            });
        }

        // Sync main date picker
        if let Some(dt) = due_date.datetime() {
            let date = dt.date();
            self.date_picker_state.update(cx, |picker, ctx| {
                picker.set_date(date, window, ctx);
            });
        }
    }

    fn get_recurrency_unit_from_type(recurrency_type: &RecurrencyType) -> &'static str {
        match recurrency_type {
            RecurrencyType::MINUTELY => "Minute(s)",
            RecurrencyType::HOURLY => "Hour(s)",
            RecurrencyType::EveryDay => "Day(s)",
            RecurrencyType::EveryWeek => "Week(s)",
            RecurrencyType::EveryMonth => "Month(s)",
            RecurrencyType::EveryYear => "Year(s)",
            _ => "Day(s)",
        }
    }

    fn get_recurrency_end_type_string(due_date: &DueDate) -> &'static str {
        match due_date.end_type() {
            RecurrencyEndType::OnDate => "OnDate",
            RecurrencyEndType::AFTER => "After",
            RecurrencyEndType::NEVER => "Never",
        }
    }

    fn recurrency_type_from_unit(unit: &str) -> RecurrencyType {
        match unit {
            "Minute(s)" => RecurrencyType::MINUTELY,
            "Hour(s)" => RecurrencyType::HOURLY,
            "Day(s)" => RecurrencyType::EveryDay,
            "Week(s)" => RecurrencyType::EveryWeek,
            "Month(s)" => RecurrencyType::EveryMonth,
            "Year(s)" => RecurrencyType::EveryYear,
            _ => RecurrencyType::EveryDay,
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
            self.date_picker_state.update(cx, |picker, ctx| {
                picker.set_date(date, window, ctx);
            });
        }
        cx.notify();
    }

    // DatePicker main handler (sets primary due date)
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
        }
        cx.notify();
    }

    // DatePicker for recurrency end
    fn on_recurrency_date_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        if let Some(date_str) = date.format("%Y-%m-%d").map(|s| s.to_string()) {
            self.due_date.recurrency_end = format!("{} 23:59:59", date_str);
            cx.emit(ScheduleButtonEvent::DateSelected(format!("Repeat until: {}", date_str)));
        }
        cx.notify();
    }

    // Handle recurrency interval input changes
    fn on_recurrency_interval_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            let value = _state.read(cx).value().to_string();
            if let Ok(interval) = value.parse::<i64>() {
                self.recurrency_interval = if interval > 0 { interval } else { 1 };
                self.due_date.recurrency_interval = self.recurrency_interval;
            }
            cx.notify();
        }
    }

    // Handle recurrency count input changes
    fn on_recurrency_count_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            let value = _state.read(cx).value().to_string();
            if let Ok(count) = value.parse::<i64>() {
                self.recurrency_count = if count > 0 { count } else { 0 };
                self.due_date.recurrency_count = self.recurrency_count;
            }
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
        self.due_date.recurrency_end = String::new();
        self.due_date.recurrency_count = 0;
        self.recurrency_interval = 1;
        self.recurrency_count = 0;
        self.recurrency_unit = Self::get_recurrency_unit_from_type(&recurrency_type).to_string();
        self.recurrency_end_type = "Never".to_string();
        self.show_custom_recurrency = false;
        cx.emit(ScheduleButtonEvent::RecurrencySelected(recurrency_type));
        cx.notify();
    }

    #[allow(dead_code)]
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.due_date = DueDate::default();
        self.selected_time = None;
        self.show_custom_recurrency = false;
        self.recurrency_unit = RECURRENCY_UNITS[2].to_string(); // "Day(s)"
        self.recurrency_end_type = "Never".to_string();
        self.recurrency_interval = 1;
        self.recurrency_count = 0;
        cx.emit(ScheduleButtonEvent::Cleared);
        cx.notify();
    }

    #[allow(dead_code)]
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
            let base_text = match self.due_date.recurrency_type {
                RecurrencyType::EveryDay => "Daily".to_string(),
                RecurrencyType::EveryWeek => "Weekly".to_string(),
                RecurrencyType::EveryMonth => "Monthly".to_string(),
                RecurrencyType::EveryYear => "Yearly".to_string(),
                _ => "None".to_string(),
            };

            if !self.due_date.recurrency_end.is_empty() {
                if let Ok(end_dt) = self.due_date.recurrency_end.parse::<chrono::NaiveDateTime>() {
                    let end_date = end_dt.date().format("%b %d").to_string();
                    format!("{} (until {})", base_text, end_date)
                } else {
                    base_text
                }
            } else {
                base_text
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
            .unwrap_or_else(|| DEFAULT_TIME.to_string())
    }

    fn sync_selected_time_from_due_date(&mut self) {
        self.selected_time =
            self.due_date.datetime().map(|dt| dt.time().format("%H:%M").to_string());
    }
}

impl Render for ScheduleButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let date_picker = self.date_picker_state.clone();
        let recurrency_date_picker = self.recurrency_date_picker_state.clone();
        let entity = cx.entity();
        let show_popover = self.show_popover;
        let show_custom_recurrency = self.show_custom_recurrency;
        let time_text = self.get_time_text();
        let repeat_text = self.get_repeat_text();
        let background_color = cx.theme().background;
        let border_color = cx.theme().border;
        let entity_id = cx.entity_id();
        let recurrency_interval_input = self.recurrency_interval_input.clone();
        let recurrency_unit = self.recurrency_unit.clone();
        let recurrency_end_type = self.recurrency_end_type.clone();
        let recurrency_count_input = self.recurrency_count_input.clone();

        // Build the UI; menu items use direct callbacks (app.update_entity) to update the component
        // state
        v_flex()
            .child(
                Button::new(("item-schedule", entity_id))
                    .outline()
                    .tooltip("set schedule")
                    .icon(IconName::Calendar)
                    .label(SharedString::from(self.get_display_text()))
                    .on_click({
                        let ent = entity.clone();
                        move |_ev, window, app| {
                            app.update_entity(&ent, |this, cx| {
                                this.show_popover = !this.show_popover;
                                // Sync date picker when opening popover
                                if this.show_popover {
                                    // Sync date picker when opening popover
                                    if let Some(dt) = this.due_date.datetime() {
                                        let date = dt.date();
                                        this.date_picker_state.update(cx, |picker, ctx| {
                                            picker.set_date(date, window, ctx);
                                        });
                                    } else {
                                        // Set today's date if no due date
                                        let today = Local::now().naive_local().date();
                                        this.date_picker_state.update(cx, |picker, ctx| {
                                            picker.set_date(today, window, ctx);
                                        });
                                    }
                                    // Also sync recurrency date picker if needed
                                    if this.due_date.end_type() == RecurrencyEndType::OnDate
                                        && let Some(end_dt) = this.due_date.end_datetime() {
                                        let end_date = end_dt.date();
                                        this.recurrency_date_picker_state.update(cx, |picker, ctx| {
                                            picker.set_date(end_date, window, ctx);
                                        });
                                    }
                                    // Sync input fields
                                    this.recurrency_interval_input.update(cx, |input, ctx| {
                                        input.set_value(this.recurrency_interval.to_string(), window, ctx);
                                    });
                                    this.recurrency_count_input.update(cx, |input, ctx| {
                                        input.set_value(this.recurrency_count.to_string(), window, ctx);
                                    });
                                } else {
                                    // When closing popover, ensure data is synced to due_date
                                    this.due_date.recurrency_interval = this.recurrency_interval;
                                    this.due_date.recurrency_count = this.recurrency_count;
                                }
                                cx.notify();
                            });
                        }
                    }),
            )
            .when(show_popover, move |this| {
                let ent = entity.clone();
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
                        // Date & time row
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(DatePicker::new(&date_picker).small().w(px(180.)))
                                .child(
                                    Button::new(("time-dropdown", entity_id))
                                        .small()
                                        .label(format!("⏰ {}", time_text))
                                        .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                            let ent_clone = ent.clone();
                                            move |menu, _window, _cx| {
                                                TIME_OPTIONS.iter().fold(menu, |m, time| {
                                                    let e = ent_clone.clone();
                                                    let t = time.to_string();
                                                    m.item(gpui_component::menu::PopupMenuItem::new(*time).on_click(move |_, _window, app| {
                                                        let value = t.clone();
                                                        app.update_entity(&e, move |this, cx| {
                                                            this.set_time(&value, cx);
                                                            cx.notify();
                                                        });
                                                    }))
                                                })
                                            }
                                        }),
                                )
                        )
                        // Recurrency row
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new(("repeat-dropdown", entity_id))
                                        .small()
                                        .label(format!("⟳ Repeat: {}", repeat_text))
                                        .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                            let ent_clone = ent.clone();
                                            move |menu, _window, _cx| {
                                                // Create menu items from a vector for cleaner initialization
                                                let menu_items = vec![
                                                    ("Daily", RecurrencyType::EveryDay),
                                                    ("Weekly", RecurrencyType::EveryWeek),
                                                    ("Monthly", RecurrencyType::EveryMonth),
                                                    ("Yearly", RecurrencyType::EveryYear),
                                                ];

                                                // Build the menu with items from the vector
                                                let mut m = menu;
                                                for (label, recurrency_type) in menu_items {
                                                    let e = ent_clone.clone();
                                                    let recurrency_type_clone = recurrency_type.clone();
                                                    m = m.item(gpui_component::menu::PopupMenuItem::new(label).on_click(move |_, _window, app| {
                                                        let recurrency_type = recurrency_type_clone.clone();
                                                        app.update_entity(&e, move |this, cx| {
                                                            this.set_recurrency(recurrency_type, cx);
                                                            cx.notify();
                                                        });
                                                    }));
                                                }

                                                // Add the Custom Date item separately since it has different logic
                                                let e = ent_clone.clone();
                                                m.item(gpui_component::menu::PopupMenuItem::new("Custom Date").on_click(move |_, _window, app| {
                                                    app.update_entity(&e, move |this, cx| {
                                                        this.show_custom_recurrency = !this.show_custom_recurrency;
                                                        if this.show_custom_recurrency {
                                                            this.due_date.is_recurring = true;
                                                            this.due_date.recurrency_supported = true;
                                                            this.recurrency_end_type = "OnDate".to_string();
                                                        }
                                                        cx.notify();
                                                    });
                                                }))
                                            }
                                        }),
                                ),
                        )
                        // Inline custom recurrency panel
                        .when(show_custom_recurrency, move |this| {
                            let ent_clone = ent.clone();
                            this.child(
                                div()
                                    .border_t_1()
                                    .border_color(border_color)
                                    .pt_3()
                                    .pb_3()
                                    .px_3()
                                    .child(
                                        v_flex()
                                            .gap_3()
                                            // Repeat every controls
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .child(div().text_sm().text_color(cx.theme().foreground).child("Repeat every"))
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                NumberInput::new(&recurrency_interval_input)
                                                                    .w(px(20.))
                                                            )
                                                            .child(div().text_sm().text_color(cx.theme().foreground).child(format!("({})", self.recurrency_interval)))
                                                            .child(
                                                                Button::new(("unit-dropdown", entity_id))
                                                                    .small()
                                                                    .label(recurrency_unit.clone())
                                                                    .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                                                        let ent_unit = ent_clone.clone();
                                                                        move |menu, _window, _cx| {
                                                                            RECURRENCY_UNITS.iter().fold(menu, |m, unit| {
                                                                                let e = ent_unit.clone();
                                                                                let u = unit.to_string();
                                                                                m.item(gpui_component::menu::PopupMenuItem::new(*unit).on_click(move |_, _window, app| {
                                                                                    let u_clone = u.clone();
                                                                                    app.update_entity(&e, move |this, cx| {
                                                                                        this.recurrency_unit = u_clone.clone();
                                                                                        let recurrency_type = Self::recurrency_type_from_unit(&u_clone);
                                                                                        this.due_date.recurrency_type = recurrency_type;
                                                                                        cx.notify();
                                                                                    });
                                                                                }))
                                                                            })
                                                                        }
                                                                    }),
                                                            ),
                                                    ),
                                            )
                                            // End controls
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .child(div().text_sm().text_color(cx.theme().foreground).child("End"))
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .child(
                                                                Button::new(("end-never", entity_id))
                                                                    .small()
                                                                    .label("Never")
                                                                    .when(recurrency_end_type == "Never", |btn| btn.primary())
                                                                    .on_click({
                                                                        let e = ent_clone.clone();
                                                                        move |_, _window, app| {
                                                                            app.update_entity(&e, move |this, cx| {
                                                                                this.recurrency_end_type = "Never".to_string();
                                                                                this.due_date.recurrency_end = String::new();
                                                                                this.due_date.recurrency_count = 0;
                                                                                cx.notify();
                                                                            });
                                                                        }
                                                                    }),
                                                            )
                                                            .child(
                                                                Button::new(("end-ondate", entity_id))
                                                                    .small()
                                                                    .label("On Date")
                                                                    .when(recurrency_end_type == "OnDate", |btn| btn.primary())
                                                                    .on_click({
                                                                        let e = ent_clone.clone();
                                                                        move |_, window, app| {
                                                                            app.update_entity(&e, move |this, cx| {
                                                                                this.recurrency_end_type = "OnDate".to_string();
                                                                                this.due_date.recurrency_count = 0;
                                                                                let date = this.due_date.datetime().map(|dt| dt.date()).unwrap_or_else(|| Local::now().naive_local().date());
                                                                                this.recurrency_date_picker_state.update(cx, |picker, ctx| { picker.set_date(date, window, ctx); });
                                                                                cx.notify();
                                                                            });
                                                                        }
                                                                    }),
                                                            )
                                                            .child(
                                                                Button::new(("end-after", entity_id))
                                                                    .small()
                                                                    .label("After")
                                                                    .when(recurrency_end_type == "After", |btn| btn.primary())
                                                                    .on_click({
                                                                        let e = ent_clone.clone();
                                                                        move |_, _window, app| {
                                                                            app.update_entity(&e, move |this, cx| {
                                                                                this.recurrency_end_type = "After".to_string();
                                                                                this.due_date.recurrency_end = String::new();
                                                                                cx.notify();
                                                                            });
                                                                        }
                                                                    }),
                                                            ),
                                                    ),
                                            )
                                            // Conditional panels for OnDate / After
                                            .when(recurrency_end_type == "OnDate", move |this| {
                                                this.child(DatePicker::new(&recurrency_date_picker).small().w(px(280.)))
                                            })
                                            .when(recurrency_end_type == "After", move |this| {
                                                this.child(h_flex().gap_2().items_center().child(div().text_sm().text_color(cx.theme().foreground).child("Occurrences:")).child(NumberInput::new(&recurrency_count_input).w(px(20.))).child(div().text_sm().text_color(cx.theme().foreground).child(format!("({})", self.recurrency_count))))
                                            }),
                                    ),
                            )
                        })
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
