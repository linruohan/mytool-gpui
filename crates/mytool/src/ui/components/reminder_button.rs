use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, ParentElement, Render, SharedString, Styled,
    Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    menu::{DropdownMenu, PopupMenuItem},
    v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::ReminderModel;

use crate::{
    create_button_wrapper, impl_button_state_base,
    todo_actions::{add_reminder, delete_reminder},
    ui::components::{PopoverListMixin, create_list_item_element},
};

pub type ReminderResult<T> = Result<T, ReminderError>;

#[derive(Debug, Clone)]
pub enum ReminderError {
    InvalidDate(String),
    InvalidTime(String),
    ParseError(String),
}

impl std::fmt::Display for ReminderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDate(msg) => write!(f, "Invalid date: {}", msg),
            Self::InvalidTime(msg) => write!(f, "Invalid time: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ReminderError {}

pub enum ReminderButtonEvent {
    Added(Arc<ReminderModel>),
    Removed(String),
    Error(Box<dyn std::error::Error + Send + Sync>),
}

pub struct ReminderButtonState {
    focus_handle: FocusHandle,
    pub item_id: String,
    items: PopoverListMixin<Arc<ReminderModel>>,
    date_picker_state: Entity<DatePickerState>,
    current_date: String,
    current_time: String,
    show_add_form: bool,
    popover_open: bool,
}

impl_button_state_base!(ReminderButtonState, ReminderButtonEvent);

impl ReminderButtonState {
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker_state = cx.new(|cx| DatePickerState::new(window, cx));

        let _ = cx.subscribe_in(&date_picker_state, window, Self::on_date_picker_event);

        let filter_fn = |reminder: &Arc<ReminderModel>, _query: &str| {
            reminder
                .due
                .as_ref()
                .map(|d| d.to_lowercase().contains(&_query.to_lowercase()))
                .unwrap_or(false)
        };

        Self {
            focus_handle: cx.focus_handle(),
            item_id,
            items: PopoverListMixin::new(filter_fn),
            date_picker_state,
            current_date: String::new(),
            current_time: "09:00".to_string(),
            show_add_form: false,
            popover_open: false,
        }
    }

    pub fn set_reminders(&mut self, reminders: Vec<Arc<ReminderModel>>, cx: &mut Context<Self>) {
        let old_reminders = self.items.items.clone();
        let has_changed = old_reminders.len() != reminders.len()
            || old_reminders.iter().zip(reminders.iter()).any(|(a, b)| a.id != b.id);

        self.items.set_items(reminders);

        if has_changed {
            cx.notify();
        }
    }

    pub fn add_reminder(&mut self, reminder: Arc<ReminderModel>, cx: &mut Context<Self>) {
        self.items.add_item(reminder.clone());
        cx.emit(ReminderButtonEvent::Added(reminder));
        cx.notify();
    }

    pub fn remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.items.remove_item(|r| r.id == reminder_id);
        cx.emit(ReminderButtonEvent::Removed(reminder_id.to_string()));
        cx.notify();
    }

    fn on_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        self.current_date = date.format("%Y-%m-%d").unwrap_or_default().to_string();
        cx.notify();
    }

    fn on_time_select(&mut self, time: &str, cx: &mut Context<Self>) {
        self.current_time = time.to_string();
        cx.notify();
    }

    fn get_time_options() -> Vec<&'static str> {
        vec!["09:00", "12:00", "17:30", "20:00"]
    }

    fn on_add_reminder(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.try_add_reminder(cx) {
            cx.emit(ReminderButtonEvent::Error(Box::new(e)));
        }
    }

    fn try_add_reminder(&mut self, cx: &mut Context<Self>) -> ReminderResult<()> {
        if self.current_date.is_empty() {
            return Err(ReminderError::InvalidDate("Date is required".to_string()));
        }

        let due_str = format!("{} {}:00", self.current_date, self.current_time);

        let reminder = ReminderModel {
            id: Uuid::new_v4().to_string(),
            item_id: Some(self.item_id.clone()),
            due: Some(due_str),
            reminder_type: Some("time".to_string()),
            ..Default::default()
        };

        self.add_reminder(Arc::new(reminder.clone()), cx);
        add_reminder(reminder, cx);

        self.current_date.clear();
        self.current_time = "09:00".to_string();
        Ok(())
    }

    fn on_remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.remove_reminder(reminder_id, cx);
        delete_reminder(reminder_id.to_string(), cx);
    }

    fn get_filtered_reminders(&self) -> Vec<Arc<ReminderModel>> {
        self.items.get_filtered("")
    }
}

impl Render for ReminderButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let show_add_form = self.show_add_form;
        let current_time = self.current_time.clone();
        let date_picker_state = self.date_picker_state.clone();
        let filtered_reminders = self.get_filtered_reminders();

        gpui_component::popover::Popover::new("reminder-popover")
            .p_0()
            .text_sm()
            .open(self.popover_open)
            .on_open_change(cx.listener(move |this, open, _, cx| {
                this.popover_open = *open;
                if !*open {
                    this.show_add_form = false;
                }
                cx.notify();
            }))
            .trigger(
                Button::new("open-reminder-dialog").small().outline().icon(IconName::AlarmSymbolic),
            )
            .track_focus(&self.focus_handle)
            .child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .w_96()
                    // 顶部添加按钮
                    .child(
                        Button::new("add-reminder-trigger")
                            .small()
                            .primary()
                            .label("Add Reminder")
                            .icon(IconName::Plus)
                            .on_click({
                                let view = view.clone();
                                move |_event, window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.show_add_form = !this.show_add_form;
                                        if this.show_add_form && this.current_date.is_empty() {
                                            let today = chrono::Utc::now().naive_utc().date();
                                            this.date_picker_state.update(cx, |picker, cx| {
                                                picker.set_date(today, window, cx);
                                            });
                                            this.current_date =
                                                today.format("%Y-%m-%d").to_string();
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    // 添加表单（点击后显示）
                    .when(show_add_form, {
                        let date_picker = date_picker_state.clone();
                        let view = view.clone();
                        move |this| {
                            this.child(
                                gpui::div()
                                    .flex()
                                    .flex_row()
                                    .gap_1()
                                    .items_center()
                                    // 日期选择框
                                    .child(
                                        DatePicker::new(&date_picker)
                                            .cleanable(true)
                                            .w(px(140.))
                                    )
                                    // 时间选择下拉框
                                    .child(
                                        Button::new("time-dropdown")
                                            .small()
                                            .outline()
                                            .label(&current_time)
                                            .dropdown_menu({
                                                let view_for_fold = view.clone();
                                                move |this, window, _cx| {
                                                    let value = view_for_fold.clone();
                                                    Self::get_time_options()
                                                        .into_iter()
                                                        .fold(this, move |this, time| {
                                                            let view_for_item = value.clone();
                                                            let view_for_click = value.clone();
                                                            let time = time.to_string();
                                                            this.item(
                                                                PopupMenuItem::new(
                                                                    SharedString::from(time.clone()),
                                                                )
                                                                .on_click(window.listener_for(
                                                                    &view_for_item,
                                                                    move |_this, _event, _window, cx| {
                                                                        let v = view_for_click.clone();
                                                                        cx.update_entity(
                                                                            &v,
                                                                            |this, cx| {
                                                                                this.on_time_select(
                                                                                    &time,
                                                                                    cx,
                                                                                );
                                                                            },
                                                                        );
                                                                    },
                                                                )),
                                                            )
                                                        })
                                                }
                                            }),
                                    )
                                    // 添加按钮
                                    .child(
                                        Button::new("add-reminder")
                                            .small()
                                            .primary()
                                            .icon(IconName::Plus)
                                            .on_click({
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_add_reminder(cx);
                                                    });
                                                }
                                            }),
                                    )
                            )
                        }
                    })
                    // 已添加的 reminder 列表
                    .child(v_flex().gap_1().children(filtered_reminders.iter().enumerate().map(
                        |(idx, reminder)| {
                            let reminder_id = reminder.id.clone();
                            let view = view.clone();
                            let display_text =
                                reminder.due.clone().unwrap_or_else(|| "No date".to_string());

                            create_list_item_element(
                                idx,
                                display_text,
                                reminder_id,
                                view,
                                move |item_id: String,
                                      view: Entity<ReminderButtonState>,
                                      cx: &mut App| {
                                    cx.update_entity(&view, |this: &mut ReminderButtonState, cx| {
                                        this.on_remove_reminder(&item_id, cx);
                                    });
                                },
                            )
                        },
                    ))),
            )
    }
}

create_button_wrapper!(ReminderButton, ReminderButtonState, "item-reminder");
