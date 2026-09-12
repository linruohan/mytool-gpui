use gpui::{
    Action, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    form::{field, v_form},
    input::{InputEvent, InputState, NumberInput},
    popover::Popover,
    radio::{Radio, RadioGroup},
    separator::Separator,
    v_flex,
};
use gpui_kit::assets::IconName;
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
    pub fn to_recurrency_type(self) -> RecurrencyType {
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
    pub fn to_label(self) -> &'static str {
        match self {
            Self::Days => "Day(s)",
            Self::Weeks => "Week(s)",
            Self::Months => "Month(s)",
            Self::Years => "Year(s)",
        }
    }

    fn all() -> [Self; 4] {
        [Self::Days, Self::Weeks, Self::Months, Self::Years]
    }

    fn index(self) -> usize {
        Self::all().iter().position(|&unit| unit == self).unwrap_or(0)
    }
}

/// 重复截止类型
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum RecurrencyEndOption {
    Never,
    OnDate,
    After,
}

impl RecurrencyEndOption {
    fn all() -> [Self; 3] {
        [Self::Never, Self::OnDate, Self::After]
    }

    fn to_label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::OnDate => "On Date",
            Self::After => "After",
        }
    }

    fn index(self) -> usize {
        Self::all().iter().position(|&option| option == self).unwrap_or(0)
    }
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
    pub fn to_label(self) -> &'static str {
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
    pub fn to_recurrency(self) -> (RecurrencyType, Option<&'static str>) {
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
        let interval_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("1").default_value("1").min(1.).step(1.)
        });
        let count_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("1").default_value("1").min(1.).step(1.)
        });
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
        // 判断是否为自定义模式：
        // 1. interval > 1（间隔大于1）
        // 2. 或者有 end_date（设置了截止日期）
        // 3. 或者有 end_count > 0（设置了重复次数）
        let is_custom = due_date.is_recurring
            && (due_date.recurrency_interval > 1
                || !due_date.recurrency_end.is_empty()
                || due_date.recurrency_count > 0);

        let preset = if !due_date.is_recurring {
            RecurrencyPreset::Daily
        } else if is_custom {
            RecurrencyPreset::Custom
        } else {
            RecurrencyPreset::from_recurrency_type(
                &due_date.recurrency_type,
                Some(&due_date.recurrency_weeks),
            )
        };

        self.selected_preset_index = preset as usize;
        self.interval_value =
            if due_date.is_recurring { due_date.recurrency_interval.max(1) } else { 1 };
        self.custom_unit = if due_date.is_recurring {
            RecurrencyUnit::from_recurrency_type(&due_date.recurrency_type)
        } else {
            RecurrencyUnit::Days
        };

        // 同步截止类型
        self.end_type = if !due_date.recurrency_end.is_empty() {
            RecurrencyEndOption::OnDate
        } else if due_date.recurrency_count > 0 {
            RecurrencyEndOption::After
        } else {
            RecurrencyEndOption::Never
        };

        // 同步截止日期
        self.end_date = if !due_date.recurrency_end.is_empty() {
            Some(due_date.recurrency_end.clone())
        } else {
            None
        };

        // 同步重复次数
        self.after_count =
            if due_date.recurrency_count > 0 { due_date.recurrency_count } else { 1 };

        // 更新间隔输入框
        self.interval_input.update(cx, |input, cx| {
            input.set_value(self.interval_value.to_string(), window, cx);
        });

        // 更新次数输入框
        self.count_input.update(cx, |input, cx| {
            input.set_value(self.after_count.to_string(), window, cx);
        });

        // 更新日期选择器
        if let Some(ref date_str) = self.end_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                self.end_date_picker.update(cx, |picker, cx| {
                    picker.set_date(date, window, cx);
                });
            }
        } else {
            self.end_date_picker.update(cx, |picker, cx| {
                picker.set_date(chrono::NaiveDate::default(), window, cx);
            });
        }

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

    /// 应用自定义重复设置
    fn apply_custom(&mut self, cx: &mut Context<Self>) {
        let recurrency_type = self.custom_unit.to_recurrency_type();
        let interval = self.interval_value;

        // 根据 end_type 设置截止日期相关字段
        let mut due_date_clone = DueDate {
            is_recurring: true,
            recurrency_supported: true,
            recurrency_type: recurrency_type.clone(),
            recurrency_interval: interval,
            ..Default::default()
        };

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

        v_flex()
            .gap_3()
            .p_3()
            .w(px(280.))
            .child(radio_group)
            .when(is_custom, |this| this.child(self.render_custom_panel(cx)))
            .child(Separator::horizontal())
            .child(done_button)
    }
}

impl RecurrencyForm {
    fn render_custom_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let interval_input = self.interval_input.clone();
        let count_input = self.count_input.clone();
        let end_type = self.end_type;
        let end_date_picker = self.end_date_picker.clone();
        let unit_index = self.custom_unit.index();
        let end_index = self.end_type.index();

        v_form()
            .child(
                field().label("Repeat every").child(
                    NumberInput::new(&interval_input).small().suffix(
                        RadioGroup::horizontal("recurrency-unit")
                            .selected_index(Some(unit_index))
                            .on_click(cx.listener(|this, index, _, cx| {
                                if let Some(&unit) = RecurrencyUnit::all().get(*index) {
                                    this.custom_unit = unit;
                                    cx.notify();
                                }
                            }))
                            .children(RecurrencyUnit::all().into_iter().map(|unit| {
                                Radio::new(format!("unit-{:?}", unit)).label(unit.to_label())
                            })),
                    ),
                ),
            )
            .child(
                field().label("End").child(
                    RadioGroup::horizontal("recurrency-end")
                        .selected_index(Some(end_index))
                        .on_click(cx.listener(|this, index, _, cx| {
                            if let Some(&option) = RecurrencyEndOption::all().get(*index) {
                                this.end_type = option;
                                cx.notify();
                            }
                        }))
                        .children(RecurrencyEndOption::all().into_iter().map(|option| {
                            Radio::new(format!("end-{:?}", option)).label(option.to_label())
                        })),
                ),
            )
            .when(end_type == RecurrencyEndOption::OnDate, move |this| {
                this.child(
                    field()
                        .label("On date")
                        .child(DatePicker::new(&end_date_picker).cleanable(true).w(px(200.))),
                )
            })
            .when(end_type == RecurrencyEndOption::After, move |this| {
                this.child(
                    field()
                        .label("After")
                        .child(NumberInput::new(&count_input).small().suffix("times")),
                )
            })
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
                        .small()
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
