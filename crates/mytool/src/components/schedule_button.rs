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
    input::InputState,
    menu::DropdownMenu,
    v_flex,
};
use todos::{enums::RecurrencyType, objects::DueDate};

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
    recurrency_unit: String,
    recurrency_end_type: String, // "Never", "OnDate", "After"
    recurrency_count_input: Entity<InputState>,

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
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let recurrency_date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));
        let recurrency_interval_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let recurrency_count_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));

        let _subscriptions = vec![
            cx.subscribe_in(&date_picker_state, window, Self::on_date_event),
            cx.subscribe_in(&recurrency_date_picker_state, window, Self::on_recurrency_date_event),
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
            recurrency_unit: "Day(s)".to_string(),
            recurrency_end_type: "Never".to_string(),
            recurrency_count_input,
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
            // safe to set date here because we are already in a context with `window` & `cx`
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
        self.show_custom_recurrency = false;
        cx.emit(ScheduleButtonEvent::RecurrencySelected(recurrency_type));
        cx.notify();
    }

    #[allow(dead_code)]
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.due_date = DueDate::default();
        self.selected_time = None;
        self.show_custom_recurrency = false;
        self.recurrency_unit = "Day(s)".to_string();
        self.recurrency_end_type = "Never".to_string();
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
                        move |_ev, _window, app| {
                            app.update_entity(&ent, |this, cx| {
                                this.show_popover = !this.show_popover;
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
                                                menu
                                                    .item(gpui_component::menu::PopupMenuItem::new("09:00").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("09:00", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("12:00").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("12:00", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("14:00").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("14:00", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("17:30").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("17:30", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("18:00").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("18:00", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("20:00").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_time("20:00", cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
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
                                                menu
                                                    .item(gpui_component::menu::PopupMenuItem::new("Daily").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_recurrency(RecurrencyType::EveryDay, cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("Weekly").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_recurrency(RecurrencyType::EveryWeek, cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("Monthly").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_recurrency(RecurrencyType::EveryMonth, cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("Yearly").on_click({
                                                        let e = ent_clone.clone();
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.set_recurrency(RecurrencyType::EveryYear, cx);
                                                                cx.notify();
                                                            });
                                                        }
                                                    }))
                                                    .item(gpui_component::menu::PopupMenuItem::new("Custom Date").on_click({
                                                        let e = ent_clone.clone();
                                                        // Toggle the inline custom recurrency panel. Initialization of
                                                        // the recurrency picker is done when user clicks 'On Date' to avoid
                                                        // unsafe set_date during menu draw.
                                                        move |_, _window, app| {
                                                            app.update_entity(&e, move |this, cx| {
                                                                this.show_custom_recurrency = !this.show_custom_recurrency;
                                                                if this.show_custom_recurrency {
                                                                    this.due_date.is_recurring = true;
                                                                    this.due_date.recurrency_supported = true;
                                                                    this.recurrency_end_type = "OnDate".to_string();
                                                                }
                                                                cx.notify();
                                                            });
                                                        }
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
                                                                Button::new(("interval-decr", entity_id))
                                                                    .small()
                                                                    .label("-")
                                                                    .on_click({
                                                                        let e = ent_clone.clone();
                                                                        move |_, window, app| {
                                                                            let e2 = e.clone();
                                                                            app.update_entity(&e2, move |this, cx| {
                                                                                let cur = this.recurrency_interval_input.read(cx).value().trim().parse::<i64>().unwrap_or(1);
                                                                                let newv = (cur - 1).max(1);
                                                                                this.recurrency_interval_input.update(cx, |view, ctx| { view.set_value(newv.to_string(), window, ctx); });
                                                                                this.due_date.recurrency_interval = newv;
                                                                                cx.notify();
                                                                            });
                                                                        }
                                                                    }),
                                                            )
                                                            .child(div().w(px(80.)).child(recurrency_interval_input.clone()))
                                                            .child(
                                                                Button::new(("interval-incr", entity_id))
                                                                    .small()
                                                                    .label("+")
                                                                    .on_click({
                                                                        let e = ent_clone.clone();
                                                                        move |_, window, app| {
                                                                            let e2 = e.clone();
                                                                            app.update_entity(&e2, move |this, cx| {
                                                                                let cur = this.recurrency_interval_input.read(cx).value().trim().parse::<i64>().unwrap_or(1);
                                                                                let newv = (cur + 1).max(1);
                                                                                this.recurrency_interval_input.update(cx, |view, ctx| { view.set_value(newv.to_string(), window, ctx); });
                                                                                this.due_date.recurrency_interval = newv;
                                                                                cx.notify();
                                                                            });
                                                                        }
                                                                    }),
                                                            )
                                                            .child(
                                                                Button::new(("unit-dropdown", entity_id))
                                                                    .small()
                                                                    .label(recurrency_unit.clone())
                                                                    .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                                                        let ent_unit = ent_clone.clone();
                                                                        move |menu, _window, _cx| {
                                                                            menu
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Minute(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Minute(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Hour(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Hour(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Day(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Day(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Week(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Week(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Month(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Month(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
                                                                                .item(gpui_component::menu::PopupMenuItem::new("Year(s)").on_click({
                                                                                    let e = ent_unit.clone();
                                                                                    move |_, _window, app| {
                                                                                        app.update_entity(&e, move |this, cx| {
                                                                                            this.recurrency_unit = "Year(s)".to_string();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                }))
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
                                                                            // initialize the recurrency date picker safely here (we have `window` and the update `cx`)
                                                                            app.update_entity(&e, move |this, cx| {
                                                                                this.recurrency_end_type = "OnDate".to_string();
                                                                                let date = this.due_date.datetime().map(|dt| dt.date()).unwrap_or_else(|| Local::now().naive_local().date());
                                                                                // update the recurrency picker with a safe call (we are in a component update context)
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
                                                this.child(h_flex().gap_2().items_center().child(div().text_sm().text_color(cx.theme().foreground).child("Occurrences:")).child(div().w(px(80.)).child(recurrency_count_input.clone())))
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
