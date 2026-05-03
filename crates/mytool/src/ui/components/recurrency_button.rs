use gpui::{
    Action, Anchor, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, div, prelude::FluentBuilder,
    px,
};
use gpui_component::{
    IconName, Side, Sizable,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    menu::DropdownMenu,
    theme::ActiveTheme,
    v_flex,
};
use serde::Deserialize;
use todos::{DueDate, enums::RecurrencyType};

use crate::{create_complex_button, impl_button_state_base};

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
#[derive(Clone, PartialEq)]
pub enum RecurrencyUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl RecurrencyUnit {
    pub fn to_recurrency_type(&self) -> RecurrencyType {
        match self {
            RecurrencyUnit::Minutes => RecurrencyType::MINUTELY,
            RecurrencyUnit::Hours => RecurrencyType::HOURLY,
            RecurrencyUnit::Days => RecurrencyType::EveryDay,
            RecurrencyUnit::Weeks => RecurrencyType::EveryWeek,
            RecurrencyUnit::Months => RecurrencyType::EveryMonth,
            RecurrencyUnit::Years => RecurrencyType::EveryYear,
        }
    }

    pub fn from_recurrency_type(recurrency_type: &RecurrencyType) -> Self {
        match recurrency_type {
            RecurrencyType::MINUTELY => RecurrencyUnit::Minutes,
            RecurrencyType::HOURLY => RecurrencyUnit::Hours,
            RecurrencyType::EveryDay => RecurrencyUnit::Days,
            RecurrencyType::EveryWeek => RecurrencyUnit::Weeks,
            RecurrencyType::EveryMonth => RecurrencyUnit::Months,
            RecurrencyType::EveryYear => RecurrencyUnit::Years,
            RecurrencyType::NONE => RecurrencyUnit::Days,
        }
    }

    pub fn to_label(&self) -> &'static str {
        match self {
            RecurrencyUnit::Minutes => "Minutes",
            RecurrencyUnit::Hours => "Hours",
            RecurrencyUnit::Days => "Day(s)",
            RecurrencyUnit::Weeks => "Week(s)",
            RecurrencyUnit::Months => "Month(s)",
            RecurrencyUnit::Years => "Year(s)",
        }
    }

    pub fn all_options() -> Vec<Self> {
        vec![
            RecurrencyUnit::Minutes,
            RecurrencyUnit::Hours,
            RecurrencyUnit::Days,
            RecurrencyUnit::Weeks,
            RecurrencyUnit::Months,
            RecurrencyUnit::Years,
        ]
    }
}

/// 重复截止类型
#[derive(Clone, PartialEq)]
pub enum RecurrencyEndOption {
    Never,
    OnDate,
    After,
}

/// 自定义重复设置
#[derive(Clone)]
pub struct CustomRecurrencySettings {
    /// 重复间隔数值
    pub interval: i64,
    /// 重复单位
    pub unit: RecurrencyUnit,
    /// 截止类型
    pub end_type: RecurrencyEndOption,
    /// 截止日期（当 end_type 为 OnDate 时）
    pub end_date: Option<String>,
    /// 重复次数（当 end_type 为 After 时）
    pub after_count: i64,
}

impl Default for CustomRecurrencySettings {
    fn default() -> Self {
        Self {
            interval: 1,
            unit: RecurrencyUnit::Days,
            end_type: RecurrencyEndOption::Never,
            end_date: None,
            after_count: 1,
        }
    }
}

/// 重复按钮状态
pub struct RecurrencyButtonState {
    focus_handle: FocusHandle,
    /// 当前选中的重复类型
    selected_recurrency: RecurrencyOption,
    /// 自定义重复设置
    custom_settings: CustomRecurrencySettings,
    /// 是否显示自定义设置面板
    show_custom_panel: bool,
    /// 日期选择器状态（用于 OnDate 截止）
    end_date_picker: Entity<DatePickerState>,
    /// 关联的 due_date
    due_date: DueDate,
    _subscriptions: Vec<Subscription>,
}

impl_button_state_base!(RecurrencyButtonState, RecurrencyButtonEvent);

/// 重复类型选项
#[derive(Clone, PartialEq)]
pub enum RecurrencyOption {
    Daily,
    Weekdays,
    Weekends,
    Weekly,
    Monthly,
    Yearly,
    None,
    Custom,
}

impl RecurrencyOption {
    pub fn to_label(&self) -> &'static str {
        match self {
            RecurrencyOption::Daily => "Daily",
            RecurrencyOption::Weekdays => "Weekdays",
            RecurrencyOption::Weekends => "Weekends",
            RecurrencyOption::Weekly => "Weekly",
            RecurrencyOption::Monthly => "Monthly",
            RecurrencyOption::Yearly => "Yearly",
            RecurrencyOption::None => "None",
            RecurrencyOption::Custom => "Custom",
        }
    }
}

impl RecurrencyButtonState {
    /// 创建新的重复按钮状态
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let end_date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let _subscriptions =
            vec![cx.subscribe_in(&end_date_picker, window, Self::on_end_date_picker_event)];

        Self {
            focus_handle: cx.focus_handle(),
            selected_recurrency: RecurrencyOption::None,
            custom_settings: CustomRecurrencySettings::default(),
            show_custom_panel: false,
            end_date_picker,
            due_date: DueDate::default(),
            _subscriptions,
        }
    }

    /// 获取当前重复设置的显示文本
    pub fn get_display_text(&self) -> String {
        match &self.selected_recurrency {
            RecurrencyOption::None => "Repeat".to_string(),
            RecurrencyOption::Daily => "Daily".to_string(),
            RecurrencyOption::Weekdays => "Weekdays".to_string(),
            RecurrencyOption::Weekends => "Weekends".to_string(),
            RecurrencyOption::Weekly => "Weekly".to_string(),
            RecurrencyOption::Monthly => "Monthly".to_string(),
            RecurrencyOption::Yearly => "Yearly".to_string(),
            RecurrencyOption::Custom => {
                let unit_label = self.custom_settings.unit.to_label();
                format!("Every {} {}", self.custom_settings.interval, unit_label)
            },
        }
    }

    /// 处理重复类型选择动作
    fn on_select_recurrency(
        &mut self,
        action: &RecurrencyAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action.0.as_str() {
            "daily" => self.apply_preset_recurrency(RecurrencyOption::Daily, cx),
            "weekdays" => self.apply_preset_recurrency(RecurrencyOption::Weekdays, cx),
            "weekends" => self.apply_preset_recurrency(RecurrencyOption::Weekends, cx),
            "weekly" => self.apply_preset_recurrency(RecurrencyOption::Weekly, cx),
            "monthly" => self.apply_preset_recurrency(RecurrencyOption::Monthly, cx),
            "yearly" => self.apply_preset_recurrency(RecurrencyOption::Yearly, cx),
            "none" => self.clear_recurrency(cx),
            "custom" => {
                self.show_custom_panel = true;
                cx.notify();
            },
            // 自定义单位选择
            s if s.starts_with("unit_") => {
                let unit_idx = s.strip_prefix("unit_").unwrap_or("0").parse::<usize>().unwrap_or(0);
                let units = RecurrencyUnit::all_options();
                if unit_idx < units.len() {
                    self.custom_settings.unit = units[unit_idx].clone();
                    cx.notify();
                }
            },
            // 间隔增减
            "interval_minus" => {
                if self.custom_settings.interval > 1 {
                    self.custom_settings.interval -= 1;
                    cx.notify();
                }
            },
            "interval_plus" => {
                self.custom_settings.interval += 1;
                cx.notify();
            },
            // 重复次数增减
            "after_minus" => {
                if self.custom_settings.after_count > 1 {
                    self.custom_settings.after_count -= 1;
                    cx.notify();
                }
            },
            "after_plus" => {
                self.custom_settings.after_count += 1;
                cx.notify();
            },
            // 截止类型选择
            "end_never" => self.select_end_type(RecurrencyEndOption::Never, cx),
            "end_on_date" => self.select_end_type(RecurrencyEndOption::OnDate, cx),
            "end_after" => self.select_end_type(RecurrencyEndOption::After, cx),
            // 应用自定义设置
            "custom_apply" => self.apply_custom_recurrency(cx),
            "custom_cancel" => {
                self.show_custom_panel = false;
                cx.notify();
            },
            _ => {},
        }
    }

    /// 处理截止日期选择器事件
    fn on_end_date_picker_event(
        &mut self,
        _state: &Entity<DatePickerState>,
        event: &DatePickerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DatePickerEvent::Change(date) = event;
        if let Some(date_str) = date.format("%Y-%m-%d").map(|s| s.to_string()) {
            self.custom_settings.end_date = Some(date_str);
            cx.notify();
        }
    }

    /// 应用预设重复类型
    fn apply_preset_recurrency(&mut self, option: RecurrencyOption, cx: &mut Context<Self>) {
        self.selected_recurrency = option.clone();
        self.show_custom_panel = false;

        match option {
            RecurrencyOption::Daily => {
                self.due_date.recurrency_type = RecurrencyType::EveryDay;
                self.due_date.recurrency_interval = 1;
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_weeks = "".to_string();
            },
            RecurrencyOption::Weekdays => {
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                self.due_date.recurrency_weeks = "1,2,3,4,5".to_string();
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
            },
            RecurrencyOption::Weekends => {
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                self.due_date.recurrency_weeks = "0,6".to_string();
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
            },
            RecurrencyOption::Weekly => {
                self.due_date.recurrency_type = RecurrencyType::EveryWeek;
                self.due_date.recurrency_interval = 1;
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
                self.due_date.recurrency_weeks = "".to_string();
            },
            RecurrencyOption::Monthly => {
                self.due_date.recurrency_type = RecurrencyType::EveryMonth;
                self.due_date.recurrency_interval = 1;
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
            },
            RecurrencyOption::Yearly => {
                self.due_date.recurrency_type = RecurrencyType::EveryYear;
                self.due_date.recurrency_interval = 1;
                self.due_date.is_recurring = true;
                self.due_date.recurrency_supported = true;
            },
            _ => {},
        }

        // 预设类型默认不设置截止
        self.due_date.recurrency_end = "".to_string();
        self.due_date.recurrency_count = 0;

        cx.emit(RecurrencyButtonEvent::RecurrencyChanged(self.due_date.clone()));
        cx.notify();
    }

    /// 应用自定义重复设置
    fn apply_custom_recurrency(&mut self, cx: &mut Context<Self>) {
        let unit_type = self.custom_settings.unit.to_recurrency_type();

        self.due_date.recurrency_type = unit_type.clone();
        self.due_date.recurrency_interval = self.custom_settings.interval.max(1);
        self.due_date.is_recurring = unit_type != RecurrencyType::NONE;
        self.due_date.recurrency_supported = unit_type != RecurrencyType::NONE;
        self.due_date.recurrency_weeks = "".to_string();

        // 应用截止设置
        match self.custom_settings.end_type {
            RecurrencyEndOption::Never => {
                self.due_date.recurrency_end = "".to_string();
                self.due_date.recurrency_count = 0;
            },
            RecurrencyEndOption::OnDate => {
                if let Some(end_date) = &self.custom_settings.end_date {
                    self.due_date.recurrency_end = format!("{} 00:00:00", end_date);
                    self.due_date.recurrency_count = 0;
                }
            },
            RecurrencyEndOption::After => {
                self.due_date.recurrency_count = self.custom_settings.after_count.max(1);
                self.due_date.recurrency_end = "".to_string();
            },
        }

        self.show_custom_panel = false;
        cx.emit(RecurrencyButtonEvent::RecurrencyChanged(self.due_date.clone()));
        cx.notify();
    }

    /// 清除重复设置
    fn clear_recurrency(&mut self, cx: &mut Context<Self>) {
        self.selected_recurrency = RecurrencyOption::None;
        self.due_date.recurrency_type = RecurrencyType::NONE;
        self.due_date.recurrency_interval = 0;
        self.due_date.is_recurring = false;
        self.due_date.recurrency_supported = false;
        self.due_date.recurrency_end = "".to_string();
        self.due_date.recurrency_count = 0;
        self.due_date.recurrency_weeks = "".to_string();
        self.show_custom_panel = false;

        cx.emit(RecurrencyButtonEvent::Cleared);
        cx.notify();
    }

    /// 设置 due_date（从外部加载已有重复设置）
    pub fn set_due_date(&mut self, due_date: DueDate, cx: &mut Context<Self>) {
        self.due_date = due_date.clone();

        // 根据 due_date 恢复选中的重复类型
        if !due_date.is_recurring {
            self.selected_recurrency = RecurrencyOption::None;
        } else {
            match due_date.recurrency_type {
                RecurrencyType::EveryDay => {
                    if due_date.recurrency_weeks == "1,2,3,4,5" {
                        self.selected_recurrency = RecurrencyOption::Weekdays;
                    } else if due_date.recurrency_weeks == "0,6" {
                        self.selected_recurrency = RecurrencyOption::Weekends;
                    } else {
                        self.selected_recurrency = RecurrencyOption::Daily;
                    }
                },
                RecurrencyType::EveryWeek => {
                    self.selected_recurrency = RecurrencyOption::Weekly;
                },
                RecurrencyType::EveryMonth => {
                    self.selected_recurrency = RecurrencyOption::Monthly;
                },
                RecurrencyType::EveryYear => {
                    self.selected_recurrency = RecurrencyOption::Yearly;
                },
                _ => {
                    self.selected_recurrency = RecurrencyOption::Custom;
                    self.custom_settings.unit =
                        RecurrencyUnit::from_recurrency_type(&due_date.recurrency_type);
                },
            }
        }

        // 恢复自定义设置
        self.custom_settings.interval = due_date.recurrency_interval.max(1);
        self.custom_settings.unit = RecurrencyUnit::from_recurrency_type(&due_date.recurrency_type);

        if !due_date.recurrency_end.is_empty() {
            self.custom_settings.end_type = RecurrencyEndOption::OnDate;
            self.custom_settings.end_date =
                due_date.recurrency_end.split_whitespace().next().map(|s| s.to_string());
        } else if due_date.recurrency_count > 0 {
            self.custom_settings.end_type = RecurrencyEndOption::After;
            self.custom_settings.after_count = due_date.recurrency_count;
        } else {
            self.custom_settings.end_type = RecurrencyEndOption::Never;
        }

        cx.notify();
    }

    /// 获取 due_date
    pub fn due_date(&self) -> DueDate {
        self.due_date.clone()
    }

    /// 选择重复截止类型
    fn select_end_type(&mut self, end_type: RecurrencyEndOption, cx: &mut Context<Self>) {
        self.custom_settings.end_type = end_type.clone();

        match end_type {
            RecurrencyEndOption::Never => {
                self.custom_settings.end_date = None;
            },
            RecurrencyEndOption::OnDate => {
                // DatePicker 默认显示今天
            },
            RecurrencyEndOption::After => {},
        }

        cx.notify();
    }
}

impl Render for RecurrencyButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display_text = self.get_display_text();
        let unit_options = RecurrencyUnit::all_options();
        let view = cx.entity();

        v_flex()
            .on_action(cx.listener(Self::on_select_recurrency))
            .child(
                Button::new(("item-recurrency", cx.entity_id()))
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::Repeat)
                    .label(SharedString::from(display_text))
                    .tooltip("Set recurrency")
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |this, _window, _cx| {
                        this.check_side(Side::Left)
                            .min_w(px(180.))
                            .menu("Daily", Box::new(RecurrencyAction("daily".to_string())))
                            .menu("Weekdays", Box::new(RecurrencyAction("weekdays".to_string())))
                            .menu("Weekends", Box::new(RecurrencyAction("weekends".to_string())))
                            .menu("Weekly", Box::new(RecurrencyAction("weekly".to_string())))
                            .menu("Monthly", Box::new(RecurrencyAction("monthly".to_string())))
                            .menu("Yearly", Box::new(RecurrencyAction("yearly".to_string())))
                            .separator()
                            .menu("None", Box::new(RecurrencyAction("none".to_string())))
                            .menu("Custom", Box::new(RecurrencyAction("custom".to_string())))
                    }),
            )
            // 自定义重复设置面板
            .when(self.show_custom_panel, |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .p_2()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(px(6.0))
                        .bg(cx.theme().background)
                        // 重复间隔设置：Repeat every [-] 1 [+] [Day(s)▼]
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(div().text_sm().child("Repeat every"))
                                .child(
                                    Button::new("interval-minus")
                                        .small()
                                        .outline()
                                        .compact()
                                        .label("-")
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    if state.custom_settings.interval > 1 {
                                                        state.custom_settings.interval -= 1;
                                                        cx.notify();
                                                    }
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .w(px(30.0))
                                        .text_center()
                                        .child(self.custom_settings.interval.to_string()),
                                )
                                .child(
                                    Button::new("interval-plus")
                                        .small()
                                        .outline()
                                        .compact()
                                        .icon(IconName::Plus)
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.custom_settings.interval += 1;
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                // 单位下拉选择
                                .child(
                                    Button::new("unit-dropdown")
                                        .small()
                                        .outline()
                                        .label(SharedString::from(
                                            self.custom_settings.unit.to_label().to_string(),
                                        ))
                                        .dropdown_menu_with_anchor(
                                            Anchor::TopLeft,
                                            move |this, _window, _cx| {
                                                let mut menu =
                                                    this.check_side(Side::Left).min_w(px(120.));
                                                for (idx, unit) in unit_options.iter().enumerate() {
                                                    menu = menu.menu(
                                                        unit.to_label(),
                                                        Box::new(RecurrencyAction(format!(
                                                            "unit_{}",
                                                            idx
                                                        ))),
                                                    );
                                                }
                                                menu
                                            },
                                        ),
                                ),
                        )
                        // 截止设置标题
                        .child(div().text_sm().child("End"))
                        // 截止类型选择：Never | On Date | After
                        .child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("end-never")
                                        .small()
                                        .outline()
                                        .label("Never")
                                        .when(
                                            self.custom_settings.end_type
                                                == RecurrencyEndOption::Never,
                                            |this| this.primary(),
                                        )
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.select_end_type(
                                                        RecurrencyEndOption::Never,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("end-on-date")
                                        .small()
                                        .outline()
                                        .label("On Date")
                                        .when(
                                            self.custom_settings.end_type
                                                == RecurrencyEndOption::OnDate,
                                            |this| this.primary(),
                                        )
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.select_end_type(
                                                        RecurrencyEndOption::OnDate,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("end-after")
                                        .small()
                                        .outline()
                                        .label("After")
                                        .when(
                                            self.custom_settings.end_type
                                                == RecurrencyEndOption::After,
                                            |this| this.primary(),
                                        )
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.select_end_type(
                                                        RecurrencyEndOption::After,
                                                        cx,
                                                    );
                                                });
                                            }
                                        }),
                                ),
                        )
                        // On Date 截止日期选择器
                        .when(
                            self.custom_settings.end_type == RecurrencyEndOption::OnDate,
                            |this| {
                                this.child(
                                    DatePicker::new(&self.end_date_picker).cleanable(true),
                                )
                            },
                        )
                        // After 重复次数设置：After [-] 1 [+] times
                        .when(
                            self.custom_settings.end_type == RecurrencyEndOption::After,
                            |this| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .child(div().text_sm().child("After"))
                                        .child(
                                            Button::new("after-minus")
                                                .small()
                                                .outline()
                                                .compact()
                                                .label("-")
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_event, _window, cx| {
                                                        cx.update_entity(&view, |state, cx| {
                                                            if state.custom_settings.after_count > 1
                                                            {
                                                                state.custom_settings.after_count -=
                                                                    1;
                                                                cx.notify();
                                                            }
                                                        });
                                                    }
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .w(px(30.0))
                                                .text_center()
                                                .child(
                                                    self.custom_settings.after_count.to_string(),
                                                ),
                                        )
                                        .child(
                                            Button::new("after-plus")
                                                .small()
                                                .outline()
                                                .compact()
                                                .icon(IconName::Plus)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_event, _window, cx| {
                                                        cx.update_entity(&view, |state, cx| {
                                                            state.custom_settings.after_count += 1;
                                                            cx.notify();
                                                        });
                                                    }
                                                }),
                                        )
                                        .child(div().text_sm().child("times")),
                                )
                            },
                        )
                        // 应用/取消按钮
                        .child(
                            h_flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("recurrency-cancel")
                                        .small()
                                        .outline()
                                        .label("Cancel")
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.show_custom_panel = false;
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("recurrency-apply")
                                        .small()
                                        .label("Apply")
                                        .on_click({
                                            let view = view.clone();
                                            move |_event, _window, cx| {
                                                cx.update_entity(&view, |state, cx| {
                                                    state.apply_custom_recurrency(cx);
                                                });
                                            }
                                        }),
                                ),
                        ),
                )
            })
    }
}

/// 创建重复按钮组件
create_complex_button!(
    RecurrencyButton,
    RecurrencyButtonState,
    RecurrencyButtonEvent,
    "item-recurrency"
);
