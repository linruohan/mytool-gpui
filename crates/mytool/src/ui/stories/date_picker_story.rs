use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Subscription, Window,
};
use gpui_component::{
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    v_flex,
};

use super::Mytool;
use crate::section;

pub struct DatePickerStory {
    date_picker: Entity<DatePickerState>,
    date_range_picker: Entity<DatePickerState>,
    date_picker_value: Option<String>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Mytool for DatePickerStory {
    fn title() -> String {
        "DatePicker".to_string()
    }

    fn description() -> String {
        "A date picker to select a date or date range.".to_string()
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DatePickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Local::now().naive_local().date();
        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_date(now, window, cx);
            picker
        });
        let date_range_picker = cx.new(|cx| DatePickerState::range(window, cx));

        let _subscriptions = vec![
            cx.subscribe(&date_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                },
            }),
            cx.subscribe(&date_range_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                },
            }),
        ];

        Self {
            date_picker,
            date_range_picker,
            date_picker_value: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }
}

impl Focusable for DatePickerStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DatePickerStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let presets =
            vec![DateRangePreset::single("Today", chrono::Local::now().naive_local().date())];

        v_flex()
            .gap_3()
            .child(
                section("Date")
                    .max_w_md()
                    .child(DatePicker::new(&self.date_picker).cleanable(true).presets(presets)),
            )
            .child(section("Range").max_w_md().child(
                DatePicker::new(&self.date_range_picker).number_of_months(2).cleanable(true),
            ))
            .child(
                section("Value")
                    .max_w_md()
                    .child(format!("Selected: {:?}", self.date_picker_value)),
            )
    }
}
