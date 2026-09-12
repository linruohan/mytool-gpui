use std::sync::Arc;

use gpui::{
    App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, prelude::FluentBuilder,
};
use gpui_component::{
    IndexPath, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    select::{Select, SelectEvent, SelectState},
    v_flex,
};
use gpui_kit::assets::IconName;
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
    /// 时间选择
    time_select: Entity<SelectState<Vec<&'static str>>>,
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
        let current_time = "09:00";
        let time_index = Self::time_options().iter().position(|t| *t == current_time).unwrap_or(0);
        let time_select = cx.new(|cx| {
            SelectState::new(
                Self::time_options(),
                Some(IndexPath::default().row(time_index)),
                window,
                cx,
            )
        });

        let _subscriptions = vec![
            cx.subscribe_in(&date_picker, window, Self::on_date_picker_event),
            cx.subscribe_in(&time_select, window, Self::on_time_select_event),
        ];

        Self {
            parent,
            focus_handle: cx.focus_handle(),
            date_picker,
            time_select,
            current_date: String::new(),
            current_time: current_time.to_string(),
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
        if !self.current_time.is_empty() {
            let time = self.current_time.as_str();
            if let Some(option) = Self::time_options().into_iter().find(|t| *t == time) {
                self.time_select.update(cx, |select, cx| {
                    select.set_selected_value(&option, window, cx);
                });
            }
        }

        // 如果有日期，同步到日期选择器
        if !self.current_date.is_empty()
            && let Ok(date) = chrono::NaiveDate::parse_from_str(&self.current_date, "%Y-%m-%d")
        {
            self.date_picker.update(cx, |picker, cx| {
                picker.set_date(date, window, cx);
            });
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

    fn on_time_select_event(
        &mut self,
        _state: &Entity<SelectState<Vec<&'static str>>>,
        event: &SelectEvent<Vec<&'static str>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(time)) = event {
            self.current_time = time.to_string();
            self.parent.update(cx, |parent, _| {
                parent.current_time = time.to_string();
            });
            let focus_handle = self.focus_handle.clone();
            window.defer(cx, move |window, cx| {
                focus_handle.focus(window, cx);
            });
            cx.notify();
        }
    }

    fn time_options() -> Vec<&'static str> {
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
        let is_temp_id = item_id.is_empty() || item_id.starts_with("temp_");

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
            // 先更新本地状态
            parent.add_reminder_internal(Arc::new(reminder.clone()), cx);

            if is_temp_id {
                // 如果是临时 ID，将提醒添加到待保存列表
                tracing::debug!(
                    "Item ID is temporary ({}), deferring reminder save",
                    reminder.item_id.as_ref().unwrap_or(&String::new())
                );
                parent.pending_reminders.push(reminder);
            } else {
                // 如果是真实 ID，立即保存到数据库
                tracing::debug!(
                    "Item ID is real ({}), saving reminder immediately",
                    reminder.item_id.as_ref().unwrap_or(&String::new())
                );
                add_reminder(reminder, cx);
            }
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
        let time_select = self.time_select.clone();

        v_flex()
            .gap_2()
            .w_full()
            .child(DatePicker::new(&date_picker).cleanable(true).w_full())
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(Select::new(&time_select).small().placeholder("09:00").flex_1())
                    .child(
                        Button::new("add-reminder").small().primary().icon(IconName::Plus).on_click({
                            let view = cx.entity();
                            move |_event, _window, cx| {
                                cx.update_entity(&view, |this, cx| {
                                    this.on_add_reminder(cx);
                                });
                            }
                        }),
                    ),
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
    /// 待保存的提醒列表（当 item_id 从临时 ID 变为真实 ID 后保存）
    pending_reminders: Vec<ReminderModel>,
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
            pending_reminders: Vec::new(),
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

    /// 更新 item_id（用于临时ID变为真实ID时）
    pub fn update_item_id(&mut self, new_item_id: String, cx: &mut Context<Self>) {
        if self.item_id != new_item_id {
            let old_id = self.item_id.clone();
            tracing::debug!(
                "ReminderButtonState: updating item_id from {} to {}",
                old_id,
                new_item_id
            );
            self.item_id = new_item_id.clone();

            // 如果有待保存的提醒，现在保存它们
            if !self.pending_reminders.is_empty() {
                tracing::debug!(
                    "Saving {} pending reminders with new item_id: {}",
                    self.pending_reminders.len(),
                    new_item_id
                );

                // 取出待保存的提醒
                let pending = std::mem::take(&mut self.pending_reminders);

                // 更新每个提醒的 item_id 并保存
                for mut reminder in pending {
                    reminder.item_id = Some(new_item_id.clone());
                    add_reminder(reminder, cx);
                }
            }

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
}

impl Render for ReminderButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let show_add_form = self.show_add_form;
        let form = self.form.clone();
        let reminders = self.items.items.clone();

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
            .trigger({
                let mut button = Button::new("open-reminder-dialog")
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::AlarmSymbolic)
                    .tooltip("提醒");
                if !reminders.is_empty() {
                    button = button.label(format!("{}", reminders.len()));
                }
                button
            })
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
                            .label("添加提醒")
                            .icon(IconName::Plus)
                            .on_click({
                                let view = view.clone();
                                move |_event, window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.show_add_form = !this.show_add_form;
                                        if this.show_add_form {
                                            this.form.update(cx, |form, cx| {
                                                form.sync_from_parent(this, window, cx);
                                                if this.current_date.is_empty() {
                                                    form.set_default_date(window, cx);
                                                }
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
                        reminders.iter().enumerate().map(|(idx, reminder)| {
                            let reminder_id = reminder.id.clone();
                            let view = view.clone();
                            let display_text = reminder
                                .due
                                .clone()
                                .unwrap_or_else(|| "无日期".to_string());

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
