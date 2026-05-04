use std::sync::Arc;

use gpui::{
    App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::FluentBuilder,
    px,
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

/// 提醒设置表单
/// 负责管理提醒添加表单的 UI 和交互逻辑
pub struct ReminderForm {
    /// 父组件引用
    parent: Entity<ReminderButtonState>,
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 日期选择器
    date_picker: Entity<DatePickerState>,
    /// 当前选中的日期字符串
    current_date: String,
    /// 当前选中的时间
    current_time: String,
    /// 订阅列表
    _subscriptions: Vec<gpui::Subscription>,
}

impl ReminderForm {
    /// 创建新的提醒表单
    pub fn new(
        parent: Entity<ReminderButtonState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let date_picker = cx.new(|cx| DatePickerState::new(window, cx));

        // 设置默认时间为 09:00
        let current_time = "09:00".to_string();

        let _subscriptions =
            vec![cx.subscribe_in(&date_picker, window, Self::on_date_picker_event)];

        Self {
            parent,
            focus_handle: cx.focus_handle(),
            date_picker,
            current_date: String::new(),
            current_time,
            _subscriptions,
        }
    }

    /// 从父组件同步状态
    pub fn sync_from_parent(
        &mut self,
        parent: &ReminderButtonState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current_date = parent.current_date.clone();
        self.current_time = parent.current_time.clone();

        // 如果有日期，同步到日期选择器
        if !self.current_date.is_empty() {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&self.current_date, "%Y-%m-%d") {
                self.date_picker.update(cx, |picker, cx| {
                    picker.set_date(date, window, cx);
                });
            }
        }

        cx.notify();
    }

    /// 处理日期选择器事件
    fn on_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        self.current_date = date.format("%Y-%m-%d").unwrap_or_default().to_string();
        cx.notify();

        let focus_handle = self.focus_handle.clone();
        window.defer(cx, move |window, cx| {
            println!("Setting focus to form after date selection");
            focus_handle.focus(window, cx);
        });
    }

    /// 选择时间
    fn select_time(&mut self, time: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.current_time = time.to_string();
        // 选择时间后，延迟设置焦点到表单，防止 popover 关闭
        let focus_handle = self.focus_handle.clone();
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });
        cx.notify();
    }

    /// 获取时间选项列表
    fn get_time_options() -> Vec<&'static str> {
        vec!["09:00", "12:00", "17:30", "20:00"]
    }

    /// 处理添加提醒
    fn on_add_reminder(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.try_add_reminder(cx) {
            cx.emit(ReminderButtonEvent::Error(Box::new(e)));
        }
    }

    /// 尝试添加提醒
    fn try_add_reminder(&mut self, cx: &mut Context<Self>) -> ReminderResult<()> {
        if self.current_date.is_empty() {
            return Err(ReminderError::InvalidDate("Date is required".to_string()));
        }

        // 从父组件获取 item_id
        let item_id = self.parent.read(cx).item_id.clone();

        let due_str = format!("{} {}:00", self.current_date, self.current_time);

        let reminder = ReminderModel {
            id: Uuid::new_v4().to_string(),
            item_id: Some(item_id),
            due: Some(due_str),
            reminder_type: Some("time".to_string()),
            ..Default::default()
        };

        // 通知父组件添加提醒
        self.parent.update(cx, |parent, cx| {
            parent.add_reminder_internal(Arc::new(reminder.clone()), cx);
            add_reminder(reminder, cx);
        });

        Ok(())
    }

    /// 设置默认日期为今天
    pub fn set_default_date(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let today = chrono::Utc::now().naive_utc().date();
        self.date_picker.update(cx, |picker, cx| {
            picker.set_date(today, window, cx);
        });
        self.current_date = today.format("%Y-%m-%d").to_string();
        cx.notify();
    }
}

impl Focusable for ReminderForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for ReminderForm {}

impl EventEmitter<ReminderButtonEvent> for ReminderForm {}

impl Render for ReminderForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let date_picker = self.date_picker.clone();
        let current_time = self.current_time.clone();

        div()
            .flex()
            .flex_row()
            .gap_1()
            .items_center()
            // 日期选择框
            .child(
                DatePicker::new(&date_picker)
                    .cleanable(true)
                    .w(px(140.)),
            )
            // 时间选择下拉框
            .child(
                Button::new("time-dropdown")
                    .small()
                    .outline()
                    .label(&current_time)
                    .dropdown_menu({
                        let view = cx.entity();
                        move |this, window, _cx| {
                            let view_for_fold = view.clone();
                            Self::get_time_options()
                                .into_iter()
                                .fold(this, move |this, time| {
                                    let time = time.to_string();
                                    this.item(
                                        PopupMenuItem::new(SharedString::from(time.clone()))
                                            .on_click(window.listener_for(
                                                &view_for_fold,
                                                move |this, _event, window, cx| {
                                                    this.select_time(&time, window, cx);
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
                        let view = cx.entity();
                        move |_event, _window, cx| {
                            cx.update_entity(&view, |this, cx| {
                                this.on_add_reminder(cx);
                            });
                        }
                    }),
            )
    }
}

/// ReminderButtonState 状态管理
/// 负责管理提醒按钮的 popover 状态、提醒列表展示、添加/删除提醒等操作
pub struct ReminderButtonState {
    focus_handle: FocusHandle,
    pub item_id: String,
    /// 提醒列表项
    items: PopoverListMixin<Arc<ReminderModel>>,
    /// 表单实体
    form: Entity<ReminderForm>,
    /// 当前日期字符串
    pub current_date: String,
    /// 当前时间字符串
    pub current_time: String,
    /// 是否显示添加表单
    show_add_form: bool,
    /// 是否打开 popover
    popover_open: bool,
}

impl_button_state_base!(ReminderButtonState, ReminderButtonEvent);

impl ReminderButtonState {
    /// 创建新的提醒按钮状态
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let parent = cx.entity();
        let form = cx.new(|cx| ReminderForm::new(parent, window, cx));

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
            form,
            current_date: String::new(),
            current_time: "09:00".to_string(),
            show_add_form: false,
            popover_open: false,
        }
    }

    /// 设置提醒列表
    pub fn set_reminders(&mut self, reminders: Vec<Arc<ReminderModel>>, cx: &mut Context<Self>) {
        let old_reminders = self.items.items.clone();
        let has_changed = old_reminders.len() != reminders.len()
            || old_reminders.iter().zip(reminders.iter()).any(|(a, b)| a.id != b.id);

        self.items.set_items(reminders);

        if has_changed {
            cx.notify();
        }
    }

    /// 添加提醒（公开方法，供外部调用）
    pub fn add_reminder(&mut self, reminder: Arc<ReminderModel>, cx: &mut Context<Self>) {
        self.add_reminder_internal(reminder, cx);
    }

    /// 添加提醒（内部方法）
    pub(crate) fn add_reminder_internal(
        &mut self,
        reminder: Arc<ReminderModel>,
        cx: &mut Context<Self>,
    ) {
        self.items.add_item(reminder.clone());
        cx.emit(ReminderButtonEvent::Added(reminder));
        cx.notify();
    }

    /// 删除提醒
    pub fn remove_reminder(&mut self, reminder_id: &str, cx: &mut Context<Self>) {
        self.items.remove_item(|r| r.id == reminder_id);
        cx.emit(ReminderButtonEvent::Removed(reminder_id.to_string()));
        delete_reminder(reminder_id.to_string(), cx);
        cx.notify();
    }

    /// 获取过滤后的提醒列表
    fn get_filtered_reminders(&self) -> Vec<Arc<ReminderModel>> {
        self.items.get_filtered("")
    }
}

impl Render for ReminderButtonState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let show_add_form = self.show_add_form;
        let form = self.form.clone();
        let filtered_reminders = self.get_filtered_reminders();

        // 同步表单状态
        self.form.update(cx, |form, cx| {
            form.sync_from_parent(self, window, cx);
        });

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
            .track_focus(&form.focus_handle(cx))
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
                                            this.form.update(cx, |form, cx| {
                                                form.set_default_date(window, cx);
                                            });
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    // 添加表单（点击后显示）
                    .when(show_add_form, |this| this.child(form.clone()))
                    // 已添加的 reminder 列表
                    .child(v_flex().gap_1().children(
                        filtered_reminders.iter().enumerate().map(|(idx, reminder)| {
                            let reminder_id = reminder.id.clone();
                            let view = view.clone();
                            let display_text = reminder
                                .due
                                .clone()
                                .unwrap_or_else(|| "No date".to_string());

                            create_list_item_element(
                                idx,
                                display_text,
                                reminder_id,
                                view,
                                move |item_id: String,
                                      view: Entity<ReminderButtonState>,
                                      cx: &mut App| {
                                    cx.update_entity(&view, |this: &mut ReminderButtonState, cx| {
                                        this.remove_reminder(&item_id, cx);
                                    });
                                },
                            )
                        }),
                    )),
            )
    }
}

create_button_wrapper!(ReminderButton, ReminderButtonState, "item-reminder");
