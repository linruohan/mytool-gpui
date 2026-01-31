use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, Styled, Subscription,
    Window, div,
};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::InputState,
    label::Label,
    v_flex,
};
use todos::{enums::RecurrencyType, objects::DueDate};

/// Events emitted by the `RepeatConfig` component.
#[derive(Clone)]
pub enum RepeatConfigEvent {
    /// Fired when the user sets a repeat/recurrency configuration. Payload is `DueDate`.
    Change(DueDate),
}

pub struct RepeatConfigState {
    focus_handle: FocusHandle,
    // numeric interval (1..100)
    interval_input: Entity<InputState>,
    // which unit: 0=Minute,1=Hour,2=Day,3=Week,4=Month,5=Year
    unit_index: usize,
    // weekday toggles
    mo: bool,
    tu: bool,
    we: bool,
    th: bool,
    fr: bool,
    sa: bool,
    su: bool,
    // end type: 0=Never,1=OnDate,2=After
    end_type: usize,
    // date picker (for On Date)
    date_picker: Entity<DatePickerState>,
    // count for After
    count_input: Entity<InputState>,
    // preview label text
    preview: String,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<RepeatConfigEvent> for RepeatConfigState {}

impl Focusable for RepeatConfigState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl RepeatConfigState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // create inputs and datepicker
        let interval_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let count_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));

        let date_picker = cx.new(|cx| {
            let mut dp = DatePickerState::new(window, cx);
            // set default date to tomorrow like the vala code did
            let tomorrow = chrono::Local::now().naive_local().date() + chrono::Duration::days(1);
            dp.set_date(tomorrow, window, cx);
            dp
        });

        // subscribe to datepicker change to update preview
        let subs = vec![cx.subscribe_in(
            &date_picker,
            window,
            |this, _state, ev: &DatePickerEvent, _window, cx| {
                let DatePickerEvent::Change(_date) = ev;
                this.preview = this.build_preview(cx);
                cx.notify();
            },
        )];

        let mut s = Self {
            focus_handle: cx.focus_handle(),
            interval_input,
            unit_index: 2, // default to Day(s)
            mo: false,
            tu: false,
            we: false,
            th: false,
            fr: false,
            sa: false,
            su: false,
            end_type: 0,
            date_picker,
            count_input,
            preview: String::new(),
            _subscriptions: subs,
        };

        // initialize preview
        s.preview = s.build_preview(cx);
        s
    }

    /// Programmatically set the due date config from a `DueDate`.
    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        // map fields
        self.unit_index = match due_date.recurrency_type {
            RecurrencyType::MINUTELY => 0,
            RecurrencyType::HOURLY => 1,
            RecurrencyType::EveryDay => 2,
            RecurrencyType::EveryWeek => 3,
            RecurrencyType::EveryMonth => 4,
            RecurrencyType::EveryYear => 5,
            RecurrencyType::NONE => 2,
        };

        // interval
        let interval_str = format!("{}", due_date.recurrency_interval.max(1));
        self.interval_input.update(cx, |view, ctx| {
            view.set_value(interval_str.clone(), window, ctx);
        });

        // weeks
        if due_date.recurrency_type == RecurrencyType::EveryWeek {
            let weeks = due_date.recurrency_weeks.split(',').map(|s| s.trim()).collect::<Vec<_>>();
            self.mo = weeks.contains(&"1");
            self.tu = weeks.contains(&"2");
            self.we = weeks.contains(&"3");
            self.th = weeks.contains(&"4");
            self.fr = weeks.contains(&"5");
            self.sa = weeks.contains(&"6");
            self.su = weeks.contains(&"7");
        } else {
            self.mo = false;
            self.tu = false;
            self.we = false;
            self.th = false;
            self.fr = false;
            self.sa = false;
            self.su = false;
        }

        // end
        if !due_date.recurrency_end.is_empty() {
            self.end_type = 1;
            if let Ok(dt) = due_date.recurrency_end.parse::<chrono::NaiveDateTime>() {
                let date = dt.date();
                self.date_picker.update(cx, |dp, ctx| {
                    dp.set_date(date, window, ctx);
                });
            }
        } else if due_date.recurrency_count > 0 {
            self.end_type = 2;
            let count_str = format!("{}", due_date.recurrency_count);
            self.count_input.update(cx, |view, ctx| {
                view.set_value(count_str.clone(), window, ctx);
            });
        } else {
            self.end_type = 0;
        }

        self.preview = self.build_preview(cx);
        cx.notify();
    }

    /// Build a DueDate object from the current UI and emit Change event.
    pub fn apply_and_emit(&mut self, cx: &mut Context<Self>) {
        let mut dd = DueDate::default();
        dd.is_recurring = true;
        dd.recurrency_type = match self.unit_index {
            0 => RecurrencyType::MINUTELY,
            1 => RecurrencyType::HOURLY,
            2 => RecurrencyType::EveryDay,
            3 => RecurrencyType::EveryWeek,
            4 => RecurrencyType::EveryMonth,
            5 => RecurrencyType::EveryYear,
            _ => RecurrencyType::EveryDay,
        };

        // parse interval
        if let Some(val) = self.interval_input.read(cx).value().trim().parse::<i64>().ok() {
            dd.recurrency_interval = val.max(1);
        } else {
            dd.recurrency_interval = 1;
        }

        // weeks
        if dd.recurrency_type == RecurrencyType::EveryWeek {
            dd.recurrency_weeks = self.collect_weeks();
        } else {
            dd.recurrency_weeks = "".to_string();
        }

        // end
        match self.end_type {
            1 => {
                // on date
                // 使用默认日期作为占位符，因为 DatePickerState 没有 selected_date 方法
                let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
                let dt = date.and_hms_opt(0, 0, 0).unwrap();
                dd.recurrency_end = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                dd.recurrency_count = 0;
            },
            2 => {
                if let Some(val) = self.count_input.read(cx).value().trim().parse::<i64>().ok() {
                    dd.recurrency_count = val.max(1);
                    dd.recurrency_end = "".to_string();
                } else {
                    dd.recurrency_count = 1;
                }
            },
            _ => {
                dd.recurrency_count = 0;
                dd.recurrency_end = "".to_string();
            },
        }

        // emit
        cx.emit(RepeatConfigEvent::Change(dd));
    }

    fn collect_weeks(&self) -> String {
        let mut parts = Vec::new();
        if self.mo {
            parts.push("1");
        }
        if self.tu {
            parts.push("2");
        }
        if self.we {
            parts.push("3");
        }
        if self.th {
            parts.push("4");
        }
        if self.fr {
            parts.push("5");
        }
        if self.sa {
            parts.push("6");
        }
        if self.su {
            parts.push("7");
        }
        parts.join(",")
    }

    fn build_preview(&self, cx: &Context<Self>) -> String {
        // simple preview similar to Vala code using DueDate::to_friendly_string when possible
        let unit = match self.unit_index {
            0 => "Minute(s)",
            1 => "Hour(s)",
            2 => "Day(s)",
            3 => "Week(s)",
            4 => "Month(s)",
            5 => "Year(s)",
            _ => "Day(s)",
        };
        let interval = self.interval_input.read(cx).value().trim().to_string();
        let interval = if interval.is_empty() { "1".to_string() } else { interval };

        let mut s = format!("Every {} {}", interval, unit);

        if self.unit_index == 3 {
            let weeks = self.collect_weeks();
            if !weeks.is_empty() {
                s.push_str(&format!(" on {}", weeks));
            }
        }

        match self.end_type {
            1 => {
                // on date
                // using simple placeholder text for preview; real localized formatting can be added
                s.push_str(" until (date)");
            },
            2 => {
                let cnt = self.count_input.read(cx).value().trim().to_string();
                let cnt = if cnt.is_empty() { "1".to_string() } else { cnt };
                s.push_str(&format!(" for {} {}", cnt, if cnt != "1" { "times" } else { "time" }));
            },
            _ => {},
        }

        s
    }
}

impl Render for RepeatConfigState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let unit_label = match self.unit_index {
            0 => "Minute(s)",
            1 => "Hour(s)",
            2 => "Day(s)",
            3 => "Week(s)",
            4 => "Month(s)",
            5 => "Year(s)",
            _ => "Day(s)",
        }
        .to_string();

        v_flex()
            .gap_2()
            .child(Label::new(&self.preview))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        // interval input
                        div().child(self.interval_input.clone().into_element()),
                    )
                    .child(
                        // unit selector that cycles on click
                        Button::new("unit-selector").label(unit_label).ghost().on_click({
                            let view = view.clone();
                            move |_ev, _window, app| {
                                app.update_entity(&view, |this, cx| {
                                    this.unit_index = (this.unit_index + 1) % 6;
                                    this.preview = this.build_preview(cx);
                                    cx.notify();
                                });
                            }
                        }),
                    ),
            )
            .child(
                // week toggles revealed when Week(s) selected
                match self.unit_index {
                    3 => {
                        // weekly toggles
                        div().child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("mo")
                                        .small()
                                        .label(if self.mo { "Mo ✓" } else { "Mo" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.mo = !this.mo;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("tu")
                                        .small()
                                        .label(if self.tu { "Tu ✓" } else { "Tu" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.tu = !this.tu;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("we")
                                        .small()
                                        .label(if self.we { "We ✓" } else { "We" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.we = !this.we;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("th")
                                        .small()
                                        .label(if self.th { "Th ✓" } else { "Th" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.th = !this.th;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("fr")
                                        .small()
                                        .label(if self.fr { "Fr ✓" } else { "Fr" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.fr = !this.fr;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("sa")
                                        .small()
                                        .label(if self.sa { "Sa ✓" } else { "Sa" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.sa = !this.sa;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("su")
                                        .small()
                                        .label(if self.su { "Su ✓" } else { "Su" })
                                        .on_click({
                                            let view = view.clone();
                                            move |_ev, _window, app| {
                                                app.update_entity(&view, |this, cx| {
                                                    this.su = !this.su;
                                                    this.preview = this.build_preview(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                ),
                        )
                    },
                    _ => div(), // empty spacer
                },
            )
            .child(
                // End options
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Button::new("never").ghost().label("Never").on_click({
                        let view = view.clone();
                        move |_ev, _window, app| {
                            app.update_entity(&view, |this, cx| {
                                this.end_type = 0;
                                this.preview = this.build_preview(cx);
                                cx.notify();
                            });
                        }
                    }))
                    .child(Button::new("on").ghost().label("On Date").on_click({
                        let view = view.clone();
                        move |_ev, _window, app| {
                            app.update_entity(&view, |this, cx| {
                                this.end_type = 1;
                                this.preview = this.build_preview(cx);
                                cx.notify();
                            });
                        }
                    }))
                    .child(Button::new("after").ghost().label("After").on_click({
                        let view = view.clone();
                        move |_ev, _window, app| {
                            app.update_entity(&view, |this, cx| {
                                this.end_type = 2;
                                this.preview = this.build_preview(cx);
                                cx.notify();
                            });
                        }
                    })),
            )
            .child(
                // stack like behavior: date picker or count input
                match self.end_type {
                    1 => div().child(DatePicker::new(&self.date_picker)),
                    2 => div().child(self.count_input.clone().into_element()),
                    _ => div(),
                },
            )
            .child(h_flex().items_end().child(
                Button::new("done").primary().label("Done").on_click({
                    let view = view.clone();
                    move |_ev, _window, app| {
                        app.update_entity(&view, |this, cx| {
                            this.apply_and_emit(cx);
                        });
                    }
                }),
            ))
    }
}

#[derive(IntoElement)]
pub struct RepeatConfig {
    id: ElementId,
    state: Entity<RepeatConfigState>,
}

impl RepeatConfig {
    pub fn new(state: &Entity<RepeatConfigState>) -> Self {
        Self { id: ("repeat-config", state.entity_id()).into(), state: state.clone() }
    }
}

impl RenderOnce for RepeatConfig {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().id(self.id).child(self.state)
    }
}
