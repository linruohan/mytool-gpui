use std::rc::Rc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::ReminderModel;

use crate::todo_actions::{add_reminder, delete_reminder};

pub enum ReminderButtonEvent {
    Added(Rc<ReminderModel>),
    Removed(String), // reminder id
}

pub struct ReminderButtonState {
    focus_handle: FocusHandle,
    pub reminders: Vec<Rc<ReminderModel>>,
    pub item_id: String,
    popover_open: bool,
    date_input: Entity<InputState>,
    time_input: Entity<InputState>,
    search_input: Entity<InputState>,
    search_query: String,
    current_date: String,
    current_time: String,
    show_time_dropdown: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ReminderButtonEvent> for ReminderButtonState {}

impl Focusable for ReminderButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ReminderButtonState {
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_input = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let time_input = cx.new(|cx| InputState::new(window, cx).placeholder("HH:MM"));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search reminders..."));
        let _subscriptions = vec![cx.subscribe_in(&search_input, window, Self::on_search_event)];

        Self {
            focus_handle: cx.focus_handle(),
            reminders: Vec::new(),
            item_id,
            popover_open: false,
            date_input,
            time_input,
            search_input,
            search_query: String::new(),
            current_date: String::new(),
            current_time: "09:00".to_string(),
            show_time_dropdown: false,
            _subscriptions,
        }
    }

    pub fn set_reminders(&mut self, reminders: Vec<Rc<ReminderModel>>, cx: &mut Context<Self>) {
        self.reminders = reminders;
        cx.notify();
    }

    pub fn add_reminder(&mut self, reminder: Rc<ReminderModel>, cx: &mut Context<Self>) {
        self.reminders.push(reminder.clone());
        cx.emit(ReminderButtonEvent::Added(reminder));
        cx.notify();
    }

    pub fn remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.reminders.retain(|r| r.id != reminder_id);
        cx.emit(ReminderButtonEvent::Removed(reminder_id.to_string()));
        cx.notify();
    }

    fn on_search_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            let query = self.search_input.read(cx).value().to_string();
            self.search_query = query;
            cx.notify();
        }
    }

    fn on_date_input_change(&mut self, cx: &mut Context<Self>) {
        let date = self.date_input.read(cx).value().to_string();
        self.current_date = date;
        cx.notify();
    }

    fn on_time_select(&mut self, time: &str, cx: &mut Context<Self>) {
        self.current_time = time.to_string();
        self.show_time_dropdown = false;
        cx.notify();
    }

    fn on_add_reminder(&mut self, cx: &mut Context<Self>) {
        if self.current_date.is_empty() {
            return;
        }

        let due_str = format!("{} {}:00", self.current_date, self.current_time);

        // 创建 ReminderModel
        let reminder = ReminderModel {
            id: Uuid::new_v4().to_string(),
            item_id: Some(self.item_id.clone()),
            due: Some(due_str),
            reminder_type: Some("time".to_string()),
            ..Default::default()
        };

        self.add_reminder(Rc::new(reminder.clone()), cx);

        // 保存到数据库
        add_reminder(reminder, cx);

        // 清空输入
        self.current_date.clear();
        self.current_time = "09:00".to_string();
        cx.notify();
    }

    fn on_remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.remove_reminder(reminder_id, cx);
        delete_reminder(reminder_id.to_string(), cx);
    }

    fn get_filtered_reminders(&self) -> Vec<Rc<ReminderModel>> {
        if self.search_query.is_empty() {
            self.reminders.clone()
        } else {
            self.reminders
                .iter()
                .filter(|r| {
                    r.due
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&self.search_query.to_lowercase()))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        }
    }
}

impl Render for ReminderButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let popover_open = self.popover_open;
        let show_time_dropdown = self.show_time_dropdown;
        let date_input = self.date_input.clone();
        let search_input = self.search_input.clone();
        let filtered_reminders = self.get_filtered_reminders();

        Popover::new("reminder-popover")
            .p_0()
            .text_sm()
            .open(popover_open)
            .on_open_change(cx.listener(move |this, open, _, cx| {
                this.popover_open = *open;
                if !*open {
                    this.search_query.clear();
                    this.show_time_dropdown = false;
                }
                cx.notify();
            }))
            .trigger(
                Button::new("open-reminder-dialog")
                    .small()
                    .outline()
                    .icon(IconName::AlarmSymbolic),
            )
            .track_focus(&self.focus_handle)
            .child(
                v_flex()
                    .gap_3()
                    .p_3()
                    .w_96()
                    // 搜索框
                    .child(
                        Input::new(&search_input)
                            .flex_1(),
                    )
                    // 日期和时间输入
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Input::new(&date_input)
                                    .flex_1(),
                            )
                            .child(
                                Button::new("time-dropdown")
                                    .small()
                                    .outline()
                                    .label(&self.current_time)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, _window, cx| {
                                            cx.update_entity(&view, |this, cx| {
                                                this.show_time_dropdown = !this.show_time_dropdown;
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-reminder")
                                    .small()
                                    .primary()
                                    .icon(IconName::Plus)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, _window, cx| {
                                            cx.update_entity(&view, |this, cx| {
                                                // 从 date_input 读取当前值
                                                let date = this.date_input.read(cx).value().to_string();
                                                this.current_date = date;
                                                this.on_add_reminder(cx);
                                            });
                                        }
                                    }),
                            ),
                    )
                    // 时间下拉列表
                    .when(show_time_dropdown, {
                        let view = view.clone();
                        move |this| {
                            this.child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("time-09:00").small().label("09:00").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_time_select("09:00", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new("time-12:00").small().label("12:00").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_time_select("12:00", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new("time-17:30").small().label("17:30").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_time_select("17:30", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new("time-20:00").small().label("20:00").on_click(
                                            {
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_time_select("20:00", cx);
                                                    });
                                                }
                                            },
                                        ),
                                    ),
                            )
                        }
                    })
                    // 提醒列表
                    .child(
                        v_flex()
                            .gap_2()
                            .children(filtered_reminders.iter().enumerate().map(
                                |(idx, reminder)| {
                                    let reminder_id = reminder.id.clone();
                                    let view = view.clone();

                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .justify_between()
                                        .px_2()
                                        .py_2()
                                        .border_b_1()
                                        .child(
                                            gpui_component::label::Label::new(
                                                reminder.due.clone().unwrap_or_else(|| "No date".to_string()),
                                            )
                                            .text_sm(),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "remove-reminder-dialog-{}",
                                                idx
                                            ))
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::UserTrashSymbolic)
                                            .on_click({
                                                let reminder_id = reminder_id.clone();
                                                let view = view.clone();
                                                move |_event, _window, cx| {
                                                    cx.update_entity(&view, |this, cx| {
                                                        this.on_remove_reminder(
                                                            &reminder_id,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            }),
                                        )
                                },
                            )),
                    ),
            )
    }
}

#[derive(IntoElement)]
pub struct ReminderButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ReminderButtonState>,
}

impl Sizable for ReminderButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ReminderButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ReminderButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ReminderButton {
    pub fn new(state: &Entity<ReminderButtonState>) -> Self {
        Self {
            id: ("item-reminder", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ReminderButton {
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
