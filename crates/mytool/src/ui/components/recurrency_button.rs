use gpui::{
    Action, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::FluentBuilder,
    px,
};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    radio::{Radio, RadioGroup},
    v_flex,
};
use serde::Deserialize;
use todos::{DueDate, enums::RecurrencyType};

use crate::{create_button_wrapper, impl_button_state_base};

/// 重复按钮动作
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = recurrency_button, no_json)]
struct RecurrencyAction(String);

/// 重复按钮事件
#[derive(Clone)]
pub enum RecurrencyButtonEvent {
    /// 重复设置已变更
    RecurrencyChanged(DueDate),
    /// 清除重复设置
    Cleared,
}

/// 重复单位（用于自定义重复）
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum RecurrencyUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl RecurrencyUnit {
    /// 将重复单位转换为 RecurrencyType
    pub fn to_recurrency_type(&self) -> RecurrencyType {
        match self {
            Self::Days => RecurrencyType::EveryDay,
            Self::Weeks => RecurrencyType::EveryWeek,
            Self::Months => RecurrencyType::EveryMonth,
            Self::Years => RecurrencyType::EveryYear,
        }
    }

    /// 从 RecurrencyType 转换为重复单位
    pub fn from_recurrency_type(recurrency_type: &RecurrencyType) -> Self {
        match recurrency_type {
            RecurrencyType::EveryDay => Self::Days,
            RecurrencyType::EveryWeek => Self::Weeks,
            RecurrencyType::EveryMonth => Self::Months,
            RecurrencyType::EveryYear => Self::Years,
            RecurrencyType::NONE => Self::Days,
            _ => Self::Days,
        }
    }

    /// 获取显示标签
    pub fn to_label(&self) -> &'static str {
        match self {
            Self::Days => "Day(s)",
            Self::Weeks => "Week(s)",
            Self::Months => "Month(s)",
            Self::Years => "Year(s)",
        }
    }
}

/// 重复截止类型
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum RecurrencyEndOption {
    Never,
    OnDate,
    After,
}

/// 重复类型选项（Radio 单选按钮项）
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum RecurrencyPreset {
    Daily,
    Weekdays,
    Weekends,
    Weekly,
    Monthly,
    Yearly,
    Custom,
}

impl RecurrencyPreset {
    /// 获取显示标签
    pub fn to_label(&self) -> &'static str {
        match self {
            Self::Daily => "Daily",
            Self::Weekdays => "Weekdays",
            Self::Weekends => "Weekends",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
            Self::Custom => "Custom",
        }
    }

    /// 转换为 RecurrencyType 和 weeks
    pub fn to_recurrency(&self) -> (RecurrencyType, Option<&'static str>) {
        match self {
            Self::Daily => (RecurrencyType::EveryDay, None),
            Self::Weekdays => (RecurrencyType::EveryWeek, Some("1,2,3,4,5")),
            Self::Weekends => (RecurrencyType::EveryWeek, Some("0,6")),
            Self::Weekly => (RecurrencyType::EveryWeek, None),
            Self::Monthly => (RecurrencyType::EveryMonth, None),
            Self::Yearly => (RecurrencyType::EveryYear, None),
            Self::Custom => (RecurrencyType::NONE, None),
        }
    }

    /// 从 RecurrencyType 创建
    pub fn from_recurrency_type(recurrency_type: &RecurrencyType, weeks: Option<&str>) -> Self {
        match recurrency_type {
            RecurrencyType::EveryDay => Self::Daily,
            RecurrencyType::EveryWeek => match weeks {
                Some("1,2,3,4,5") => Self::Weekdays,
                Some("0,6") => Self::Weekends,
                _ => Self::Weekly,
            },
            RecurrencyType::EveryMonth => Self::Monthly,
            RecurrencyType::EveryYear => Self::Yearly,
            RecurrencyType::NONE => Self::Custom,
            _ => Self::Daily,
        }
    }

    /// 所有预设选项及其索引
    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::Daily,
            Self::Weekdays,
            Self::Weekends,
            Self::Weekly,
            Self::Monthly,
            Self::Yearly,
            Self::Custom,
        ]
    }
}

/// 重复设置表单
pub struct RecurrencyForm {
    /// 父组件引用
    parent: Entity<RecurrencyButtonState>,
    /// 选中的预设类型索引
    selected_preset_index: usize,
    /// 自定义重复间隔数值
    interval_value: i64,
    /// 自定义重复单位
    custom_unit: RecurrencyUnit,
    /// 截止类型
    end_type: RecurrencyEndOption,
    /// 截止日期选择器
    end_date_picker: Entity<DatePickerState>,
    /// 截止日期字符串
    end_date: Option<String>,
    /// 重复次数
    after_count: i64,
    /// 间隔输入框
    interval_input: Entity<InputState>,
    /// 次数输入框
    count_input: Entity<InputState>,
    /// 订阅列表
    _subscriptions: Vec<gpui::Subscription>,
}

impl RecurrencyForm {
    /// 创建新的表单
    pub fn new(
        parent: Entity<RecurrencyButtonState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let interval_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let count_input = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let end_date_picker = cx.new(|cx| DatePickerState::new(window, cx));

        // 设置初始值
        interval_input.update(cx, |input, cx| {
            input.set_value("1", window, cx);
        });
        count_input.update(cx, |input, cx| {
            input.set_value("1", window, cx);
        });

        let _subscriptions = vec![
            cx.subscribe_in(&interval_input, window, Self::on_interval_input_event),
            cx.subscribe_in(&count_input, window, Self::on_count_input_event),
            cx.subscribe_in(&end_date_picker, window, Self::on_end_date_event),
        ];

        Self {
            parent,
            selected_preset_index: 0, // 默认选中 Daily
            interval_value: 1,
            custom_unit: RecurrencyUnit::Days,
            end_type: RecurrencyEndOption::Never,
            end_date_picker,
            end_date: None,
            after_count: 1,
            interval_input,
            count_input,
            _subscriptions,
        }
    }

    /// 从父组件同步状态
    pub fn sync_from_parent(
        &mut self,
        due_date: &DueDate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let preset = if due_date.is_recurring {
            RecurrencyPreset::from_recurrency_type(
                &due_date.recurrency_type,
                Some(&due_date.recurrency_weeks),
            )
        } else {
            RecurrencyPreset::Daily
        };

        self.selected_preset_index = preset as usize;
        self.interval_value =
            if due_date.is_recurring { due_date.recurrency_interval.max(1) } else { 1 };
        self.custom_unit = if due_date.is_recurring {
            RecurrencyUnit::from_recurrency_type(&due_date.recurrency_type)
        } else {
            RecurrencyUnit::Days
        };

        self.interval_input.update(cx, |input, cx| {
            input.set_value(&self.interval_value.to_string(), window, cx);
        });

        cx.notify();
    }

    /// 获取当前选中的预设类型
    fn get_selected_preset(&self) -> RecurrencyPreset {
        let presets = RecurrencyPreset::all_presets();
        presets.get(self.selected_preset_index).copied().unwrap_or(RecurrencyPreset::Daily)
    }

    /// 处理间隔输入事件
    fn on_interval_input_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            self.sync_interval_from_input(cx);
        }
    }

    /// 处理次数输入事件
    fn on_count_input_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            self.sync_count_from_input(cx);
        }
    }

    /// 处理结束日期选择事件
    fn on_end_date_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        if let Some(formatted) = date.format("%Y-%m-%d") {
            self.end_date = Some(formatted.to_string());
            cx.notify();
        }
    }

    /// 从输入框同步间隔值
    fn sync_interval_from_input(&mut self, cx: &mut Context<Self>) {
        self.interval_input.update(cx, |input, _| {
            if let Ok(value) = input.value().parse::<i64>() {
                self.interval_value = value.max(1);
            }
        });
    }

    /// 从输入框同步次数值
    fn sync_count_from_input(&mut self, cx: &mut Context<Self>) {
        self.count_input.update(cx, |input, _| {
            if let Ok(value) = input.value().parse::<i64>() {
                self.after_count = value.max(1);
            }
        });
    }

    /// 间隔减一
    fn decrement_interval(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.interval_value > 1 {
            self.interval_value -= 1;
            self.interval_input.update(cx, |input, cx| {
                input.set_value(&self.interval_value.to_string(), window, cx);
            });
        }
    }

    /// 间隔加一
    fn increment_interval(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.interval_value += 1;
        self.interval_input.update(cx, |input, cx| {
            input.set_value(&self.interval_value.to_string(), window, cx);
        });
    }

    /// 次数减一
    fn decrement_count(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.after_count > 1 {
            self.after_count -= 1;
            self.count_input.update(cx, |input, cx| {
                input.set_value(&self.after_count.to_string(), window, cx);
            });
        }
    }

    /// 次数加一
    fn increment_count(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.after_count += 1;
        self.count_input.update(cx, |input, cx| {
            input.set_value(&self.after_count.to_string(), window, cx);
        });
    }

    /// 应用自定义重复设置
    fn apply_custom(&mut self, cx: &mut Context<Self>) {
        let recurrency_type = self.custom_unit.to_recurrency_type();
        let interval = self.interval_value;

        // 根据 end_type 设置截止日期相关字段
        let mut due_date_clone = DueDate::default();
        due_date_clone.is_recurring = true;
        due_date_clone.recurrency_supported = true;
        due_date_clone.recurrency_type = recurrency_type.clone();
        due_date_clone.recurrency_interval = interval;

        match self.end_type {
            RecurrencyEndOption::Never => {},
            RecurrencyEndOption::OnDate => {
                if let Some(ref date_str) = self.end_date {
                    due_date_clone.recurrency_end = date_str.clone();
                }
            },
            RecurrencyEndOption::After => {
                due_date_clone.recurrency_count = self.after_count;
            },
        }

        self.parent.update(cx, |parent, cx| {
            parent.due_date = due_date_clone.clone();
            cx.emit(RecurrencyButtonEvent::RecurrencyChanged(due_date_clone));
        });

        cx.emit(DismissEvent);
    }
}

impl Focusable for RecurrencyForm {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.interval_input.focus_handle(cx)
    }
}

impl EventEmitter<DismissEvent> for RecurrencyForm {}

impl Render for RecurrencyForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_custom = self.get_selected_preset() == RecurrencyPreset::Custom;
        let selected_index = self.selected_preset_index;
        let presets = RecurrencyPreset::all_presets();

        // 构建 RadioGroup
        let radio_group =
            RadioGroup::vertical("recurrency-preset-group")
                .selected_index(Some(selected_index))
                .on_click(cx.listener(move |this, index, _, cx| {
                    this.selected_preset_index = *index;
                    cx.notify();
                }))
                .children(presets.iter().map(|preset| {
                    Radio::new(format!("preset-{:?}", preset)).label(preset.to_label())
                }));

        // 构建 Done 按钮
        let done_button = Button::new("done").w_full().primary().label("Done").on_click(
            cx.listener(move |this, _, _window, cx| {
                let preset = this.get_selected_preset();
                if preset == RecurrencyPreset::Custom {
                    this.apply_custom(cx);
                } else {
                    let (recurrency_type, weeks) = preset.to_recurrency();
                    this.parent.update(cx, |parent, cx| {
                        parent.apply_recurrency_change(Some((recurrency_type, 1, weeks)), cx);
                    });
                    cx.emit(DismissEvent);
                }
            }),
        );

        // 根据 is_custom 决定是否渲染自定义面板
        if is_custom {
            let custom_panel = self.render_custom_panel(cx);
            v_flex()
                .gap_3()
                .p_3()
                .w(px(280.))
                .child(v_flex().gap_2().child(radio_group))
                .child(custom_panel)
                .child(done_button)
        } else {
            v_flex()
                .gap_3()
                .p_3()
                .w(px(280.))
                .child(v_flex().gap_2().child(radio_group))
                .child(done_button)
        }
    }
}

impl RecurrencyForm {
    /// 渲染自定义面板
    fn render_custom_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let interval_input = self.interval_input.clone();
        let count_input = self.count_input.clone();
        let end_type = self.end_type;
        let end_date_picker = self.end_date_picker.clone();

        v_flex()
            .gap_3()
            .p_2()
            .border_1()
            .rounded_lg()
            .border_color(gpui::rgb(0xe0e0e0))
            .child(
                // Repeat every
                v_flex().gap_2().child("Repeat every").child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            h_flex()
                                .gap_0()
                                .border_1()
                                .rounded_md()
                                .border_color(gpui::rgb(0xd0d0d0))
                                .overflow_hidden()
                                .child(
                                    Button::new("interval-dec")
                                        .ghost()
                                        .compact()
                                        .label("−")
                                        .px_2()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.decrement_interval(window, cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .w(px(40.))
                                        .child(Input::new(&interval_input).appearance(false)),
                                )
                                .child(
                                    Button::new("interval-inc")
                                        .ghost()
                                        .compact()
                                        .label("+")
                                        .px_2()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.increment_interval(window, cx);
                                        })),
                                ),
                        )
                        .child(
                            // 单位选择器
                            h_flex()
                                .gap_0()
                                .border_1()
                                .rounded_md()
                                .border_color(gpui::rgb(0xd0d0d0))
                                .overflow_hidden()
                                .child(self.render_unit_button(RecurrencyUnit::Days, cx))
                                .child(self.render_unit_button(RecurrencyUnit::Weeks, cx))
                                .child(self.render_unit_button(RecurrencyUnit::Months, cx))
                                .child(self.render_unit_button(RecurrencyUnit::Years, cx)),
                        ),
                ),
            )
            .child(
                // End
                v_flex()
                    .gap_2()
                    .child("End")
                    .child(
                        h_flex()
                            .gap_0()
                            .border_1()
                            .rounded_md()
                            .border_color(gpui::rgb(0xd0d0d0))
                            .overflow_hidden()
                            .child(self.render_end_button(RecurrencyEndOption::Never, cx))
                            .child(self.render_end_button(RecurrencyEndOption::OnDate, cx))
                            .child(self.render_end_button(RecurrencyEndOption::After, cx)),
                    )
                    // On Date 日期选择器
                    .when(end_type == RecurrencyEndOption::OnDate, move |this| {
                        this.child(
                            DatePicker::new(&end_date_picker).cleanable(true).w(px(200.)),
                        )
                    })
                    // After 次数输入
                    .when(end_type == RecurrencyEndOption::After, move |this| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_0()
                                        .border_1()
                                        .rounded_md()
                                        .border_color(gpui::rgb(0xd0d0d0))
                                        .overflow_hidden()
                                        .child(
                                            Button::new("count-dec")
                                                .ghost()
                                                .compact()
                                                .label("−")
                                                .px_2()
                                                .on_click(
                                                    cx.listener(move |this, _, window, cx| {
                                                        this.decrement_count(window, cx);
                                                    }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .w(px(40.))
                                                .child(Input::new(&count_input).appearance(false)),
                                        )
                                        .child(
                                            Button::new("count-inc")
                                                .ghost()
                                                .compact()
                                                .label("+")
                                                .px_2()
                                                .on_click(
                                                    cx.listener(move |this, _, window, cx| {
                                                        this.increment_count(window, cx);
                                                    }),
                                                ),
                                        ),
                                )
                                .child("times"),
                        )
                    }),
            )
    }

    /// 渲染单位按钮
    fn render_unit_button(
        &mut self,
        unit: RecurrencyUnit,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = unit == self.custom_unit;
        let label = unit.to_label();

        Button::new(format!("unit-{:?}", unit))
            .ghost()
            .compact()
            .px_2()
            .label(label)
            .when(is_selected, |btn| btn.bg(gpui::rgb(0xe8e8e8)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.custom_unit = unit;
                cx.notify();
            }))
    }

    /// 渲染 End 类型按钮
    fn render_end_button(
        &mut self,
        end_option: RecurrencyEndOption,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = end_option == self.end_type;
        let label = match end_option {
            RecurrencyEndOption::Never => "Never",
            RecurrencyEndOption::OnDate => "On Date",
            RecurrencyEndOption::After => "After",
        };

        Button::new(format!("end-{:?}", end_option))
            .flex_1()
            .ghost()
            .compact()
            .label(label)
            .when(is_selected, |btn| btn.bg(gpui::rgb(0xe8e8e8)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.end_type = end_option;
                cx.notify();
            }))
    }
}

/// 重复按钮状态
pub struct RecurrencyButtonState {
    focus_handle: FocusHandle,
    /// 表单实体
    form: Entity<RecurrencyForm>,
    /// 是否显示弹出面板
    popover_open: bool,
    /// 当前关联的 due_date
    pub due_date: DueDate,
}

impl_button_state_base!(RecurrencyButtonState, RecurrencyButtonEvent);

impl RecurrencyButtonState {
    /// 创建新的重复按钮状态
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let parent = cx.entity();
        let form = cx.new(|cx| RecurrencyForm::new(parent, window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            form,
            popover_open: false,
            due_date: DueDate::default(),
        }
    }

    /// 设置 due_date
    pub fn set_due_date(&mut self, due_date: DueDate, window: &mut Window, cx: &mut Context<Self>) {
        let old_due_date = self.due_date.clone();
        let has_changed = old_due_date != due_date;

        self.due_date = due_date.clone();

        // 同步表单状态
        self.form.update(cx, |form, cx| {
            form.sync_from_parent(&due_date, window, cx);
        });

        if has_changed {
            cx.notify();
        }
    }

    /// 应用重复设置变更
    fn apply_recurrency_change(
        &mut self,
        recurrency: Option<(RecurrencyType, i64, Option<&str>)>,
        cx: &mut Context<Self>,
    ) {
        match recurrency {
            Some((recurrency_type, interval, weeks)) => {
                self.due_date.is_recurring = recurrency_type != RecurrencyType::NONE;
                self.due_date.recurrency_supported = recurrency_type != RecurrencyType::NONE;
                self.due_date.recurrency_type = recurrency_type.clone();
                self.due_date.recurrency_interval = interval;

                if let Some(weeks_str) = weeks {
                    self.due_date.recurrency_weeks = weeks_str.to_string();
                } else {
                    self.due_date.recurrency_weeks.clear();
                }

                cx.emit(RecurrencyButtonEvent::RecurrencyChanged(self.due_date.clone()));
            },
            None => {
                self.due_date.is_recurring = false;
                self.due_date.recurrency_type = RecurrencyType::NONE;
                self.due_date.recurrency_interval = 0;
                self.due_date.recurrency_weeks.clear();
                cx.emit(RecurrencyButtonEvent::Cleared);
            },
        }

        self.popover_open = false;
        cx.notify();
    }

    /// 获取显示文本
    fn get_display_text(&self) -> String {
        if !self.due_date.is_recurring {
            return "Repeat".to_string();
        }

        let preset = RecurrencyPreset::from_recurrency_type(
            &self.due_date.recurrency_type,
            Some(&self.due_date.recurrency_weeks),
        );
        preset.to_label().to_string()
    }
}

impl Render for RecurrencyButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let display_text = self.get_display_text();
        let form = self.form.clone();

        v_flex().child(
            Popover::new("recurrency-popover")
                .p_0()
                .text_sm()
                .open(self.popover_open)
                .on_open_change(cx.listener(|this, open, _, cx| {
                    this.popover_open = *open;
                    cx.notify();
                }))
                .trigger(
                    Button::new(("recurrency-btn", cx.entity_id()))
                        .outline()
                        .icon(IconName::RefreshCw)
                        .label(SharedString::from(display_text)),
                )
                .track_focus(&form.focus_handle(cx))
                .child(form.clone()),
        )
    }
}

create_button_wrapper!(RecurrencyButton, RecurrencyButtonState, "item-recurrency");
