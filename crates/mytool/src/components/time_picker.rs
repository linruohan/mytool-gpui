use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    Sizable, Size, StyleSized, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

/// Events emitted by the [`TimePicker`].
#[derive(Clone)]
pub enum TimePickerEvent {
    /// Fired when the selected time changes (payload: "HH:MM").
    Change(String),
    /// Fired when the input is activated/confirmed (e.g. Enter pressed).
    Activated,
}

pub struct TimePickerState {
    focus_handle: FocusHandle,
    input: Entity<InputState>,
    /// The selected time in "HH:MM" (24h) format.
    pub value: Option<String>,
    /// Whether a small dropdown of presets is visible.
    show_presets: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<TimePickerEvent> for TimePickerState {}

impl Focusable for TimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TimePickerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("HH:MM"));

        // Subscribe to input events to parse and react
        let subs =
            vec![cx.subscribe_in(&input, window, |this, state, ev: &InputEvent, _window, cx| {
                match ev {
                    InputEvent::Change => {
                        let val = state.read(cx).value();
                        // Try to parse on each change, but only accept valid times
                        if let Some(parsed) = Self::parse_time(&val) {
                            // update stored value and notify
                            this.value = Some(parsed.clone());
                            // normalize input field to parsed representation
                            state.update(cx, |s, cx| {
                                s.set_value(parsed.clone(), _window, cx);
                            });
                            cx.emit(TimePickerEvent::Change(parsed));
                        }
                    },
                    InputEvent::PressEnter { .. } => {
                        // On Enter, try to parse; if successful emit Activated as well
                        let val = state.read(cx).value();
                        if let Some(parsed) = Self::parse_time(&val) {
                            this.value = Some(parsed.clone());
                            state.update(cx, |s, cx| {
                                s.set_value(parsed.clone(), _window, cx);
                            });
                            cx.emit(TimePickerEvent::Change(parsed));
                        }
                        cx.emit(TimePickerEvent::Activated);
                    },
                    _ => {},
                }
            })];

        Self {
            focus_handle: cx.focus_handle(),
            input,
            value: None,
            show_presets: false,
            _subscriptions: subs,
        }
    }

    /// Programmatically set the time. Accepts "HH:MM" (24h) or tries to parse other formats.
    pub fn set_time(
        &mut self,
        time_str: impl AsRef<str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(parsed) = Self::parse_time(time_str.as_ref()) {
            self.value = Some(parsed.clone());
            self.input.update(cx, |view, cx| {
                view.set_value(parsed, window, cx);
            });
            cx.emit(TimePickerEvent::Change(self.value.clone().unwrap()));
            cx.notify();
        }
    }

    /// Clear the time selection and input.
    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.value = None;
        self.input.update(cx, |view, cx| {
            view.set_value("", window, cx);
        });
        cx.emit(TimePickerEvent::Change(String::new()));
        cx.notify();
    }

    /// Parse various human time inputs into "HH:MM" 24-hour format.
    ///
    /// Supports:
    /// - "9", "09" -> interprets as hour, minute = 0
    /// - "930", "0930", "9:30", "09:30"
    /// - "9am", "9:30pm", "12am", "12pm"
    /// - "2130", "21:30"
    fn parse_time(s: &str) -> Option<String> {
        let mut txt = s.trim().to_lowercase();
        if txt.is_empty() {
            return None;
        }

        // Remove spaces
        txt.retain(|c| c != ' ');

        // detect am/pm
        let mut is_pm = false;
        let mut is_am = false;
        if txt.ends_with("am") {
            is_am = true;
            txt = txt.trim_end_matches("am").to_string();
        } else if txt.ends_with("pm") {
            is_pm = true;
            txt = txt.trim_end_matches("pm").to_string();
        }

        // If contains colon, split
        let (mut hour_opt, mut min_opt): (Option<i32>, Option<i32>) = (None, None);
        if txt.contains(':') {
            let parts: Vec<&str> = txt.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(h) = parts[0].parse::<i32>() {
                    hour_opt = Some(h);
                }
                if let Ok(m) = parts[1].parse::<i32>() {
                    min_opt = Some(m);
                }
            } else {
                return None;
            }
        } else {
            // no colon, take digits
            let digits: String = txt.chars().filter(|c| c.is_ascii_digit()).collect();
            match digits.len() {
                0 => return None,
                1 | 2 => {
                    if let Ok(h) = digits.parse::<i32>() {
                        hour_opt = Some(h);
                        min_opt = Some(0);
                    }
                },
                3 => {
                    // e.g. "930" -> 9:30
                    let (h, m) = (&digits[0..1], &digits[1..3]);
                    if let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>()) {
                        hour_opt = Some(h);
                        min_opt = Some(m);
                    }
                },
                4 | _ => {
                    // take first two as hour, last two as minute (for 4+ digits)
                    let h_s = &digits[0..2];
                    let m_s = &digits[2..4];
                    if let (Ok(h), Ok(m)) = (h_s.parse::<i32>(), m_s.parse::<i32>()) {
                        hour_opt = Some(h);
                        min_opt = Some(m);
                    }
                },
            }
        }

        let mut hour = hour_opt.unwrap_or(0);
        let minute = min_opt.unwrap_or(0);

        // Apply am/pm adjustments if suffix present
        if is_am || is_pm {
            if hour < 1 || hour > 12 {
                // e.g. "0am" invalid, but allow "12am"/"12pm" semantics
                if hour == 0 {
                    hour = 12;
                } else {
                    return None;
                }
            }
            if is_am {
                if hour == 12 {
                    hour = 0;
                }
            } else if is_pm {
                if hour != 12 {
                    hour += 12;
                }
            }
        }

        // Validate ranges
        if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
            return None;
        }

        Some(format!("{:02}:{:02}", hour, minute))
    }

    /// Toggle the small preset dropdown that shows a few common times.
    pub fn toggle_presets(&mut self, cx: &mut Context<Self>) {
        self.show_presets = !self.show_presets;
        cx.notify();
    }

    fn preset_times() -> Vec<&'static str> {
        vec!["09:00", "12:00", "17:30", "20:00"]
    }
}

impl Render for TimePickerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let show_presets = self.show_presets;
        let input = self.input.clone();
        let value = self.value.clone();

        div().child(
            v_flex()
                .gap_1()
                .child(
                    h_flex().gap_2().items_center().child(Input::new(&input).flex_1()).child(
                        Button::new("time-presets-toggle")
                            .small()
                            .ghost()
                            .label(match &value {
                                Some(v) => v.clone(),
                                None => "Set".to_string(),
                            })
                            .on_click({
                                let view = view.clone();
                                move |_event, _window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.show_presets = !this.show_presets;
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
                )
                .when(show_presets, {
                    let view = view.clone();
                    move |this| {
                        this.child(v_flex().gap_1().px_2().children(
                            Self::preset_times().iter().enumerate().map(move |(idx, t)| {
                                let view = view.clone();
                                let t = t.to_string();
                                Button::new(format!("time-preset-{}", idx))
                                    .small()
                                    .outline()
                                    .label(&t)
                                    .on_click({
                                        let view = view.clone();
                                        let t = t.clone();
                                        move |_ev, window, cx| {
                                            cx.update_entity(&view, |this, cx| {
                                                // set time and emit
                                                this.value = Some(t.clone());
                                                this.input.update(cx, |s, cx| {
                                                    s.set_value(t.clone(), window, cx);
                                                });
                                                cx.emit(TimePickerEvent::Change(t.clone()));
                                                this.show_presets = false;
                                                cx.notify();
                                            });
                                        }
                                    })
                            }),
                        ))
                    }
                }),
        )
    }
}

#[derive(IntoElement)]
pub struct TimePicker {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<TimePickerState>,
}

impl Sizable for TimePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for TimePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for TimePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl TimePicker {
    pub fn new(state: &Entity<TimePickerState>) -> Self {
        Self {
            id: ("time-picker", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for TimePicker {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .child(self.state.clone())
            .input_text_size(self.size)
            .refine_style(&self.style)
    }
}
