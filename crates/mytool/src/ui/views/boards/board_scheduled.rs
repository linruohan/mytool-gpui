//! ScheduledBoard - 计划任务视图
//!
//! 显示计划中任务，在其他时间去执行的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{collections::HashMap, sync::Arc};

use chrono::Datelike;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div, prelude::FluentBuilder,
    px,
};
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    calendar::{Calendar, CalendarEvent, CalendarState},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;

use crate::{
    BoardBase, VisualHierarchy, section_with_title,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FAB_BOTTOM_PAD, FinishItemDialogStyle, render_board_header,
            show_finish_item_dialog, show_item_delete_dialog, show_pin_item_dialog,
            with_selected_item,
        },
        board_renderer,
        container_board::Board,
        reorder_in_groups,
    },
};

impl EventEmitter<BoardItemClickEvent> for ScheduledBoard {}

pub struct ScheduledBoard {
    base: BoardBase,
    /// 按日期分组的缓存（在 refresh 时构建，render 只读）
    grouped_by_date: Vec<(String, Vec<(usize, Arc<todos::entity::ItemModel>)>)>,
    date_picker: Entity<DatePickerState>,
    calendar: Entity<CalendarState>,
    filter_date: Option<String>,
    _date_subscription: Subscription,
    _calendar_subscription: Subscription,
}

impl ScheduledBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let calendar = cx.new(|cx| CalendarState::new(window, cx));
        let _date_subscription =
            cx.subscribe(&date_picker, |this, _, event: &DatePickerEvent, cx| {
                let DatePickerEvent::Change(date) = event;
                this.filter_date = date.format("%Y-%m-%d").map(|s| s.to_string());
                cx.notify();
            });
        let _calendar_subscription =
            cx.subscribe(&calendar, |this, _, event: &CalendarEvent, cx| {
                let CalendarEvent::Selected(date) = event;
                this.filter_date = date.format("%Y-%m-%d").map(|s| s.to_string());
                cx.notify();
            });
        Self {
            base: BoardBase::new(window, cx),
            grouped_by_date: Vec::new(),
            date_picker,
            calendar,
            filter_date: None,
            _date_subscription,
            _calendar_subscription,
        }
    }

    fn reorder_active(&self, delta: i32, cx: &mut Context<Self>) {
        let Some(active_index) = self.base.active_index else {
            return;
        };
        let groups: Vec<&[_]> =
            self.grouped_by_date.iter().map(|(_, items)| items.as_slice()).collect();
        let Some(updated) = reorder_in_groups(&groups, active_index, delta) else {
            return;
        };
        crate::core::actions::batch::batch_update_items(updated, cx);
    }

    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state_items) = self.base.apply_store_refresh(
            window,
            cx,
            crate::core::state::ChangeMask::affects_scheduled,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().scheduled_items_cached(cache)
            },
        ) {
            self.grouped_by_date = group_scheduled_by_date(state_items.as_slice());
        }
    }

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        self.base.show_item_dialog(window, cx, is_edit, section_id);
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }

    pub fn show_pin_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_pin_item_dialog(window, cx, item);
        });
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_finish_item_dialog(window, cx, item, FinishItemDialogStyle::Standard);
        });
    }

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        self.base.show_section_dialog(window, cx, section_id, is_edit);
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        BoardBase::show_section_delete_dialog(window, cx, section_id);
    }
}

fn format_schedule_heading(date_key: &str, today: &str) -> String {
    if date_key == "无日期" {
        return "无日期".to_string();
    }

    let Ok(date) = chrono::NaiveDate::parse_from_str(date_key, "%Y-%m-%d") else {
        return date_key.to_string();
    };
    let weekday =
        ["日", "一", "二", "三", "四", "五", "六"][date.weekday().num_days_from_sunday() as usize];
    let today_date = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok();

    if Some(date) == today_date {
        format!("今天 · 周{weekday}")
    } else if today_date.and_then(|d| d.succ_opt()) == Some(date) {
        format!("明天 · 周{weekday}")
    } else if today_date.and_then(|d| d.pred_opt()) == Some(date) {
        format!("昨天 · 周{weekday}")
    } else if date.year() == chrono::Local::now().year() {
        format!("周{weekday} · {}月{}日", date.month(), date.day())
    } else {
        format!("{}年{}月{}日", date.year(), date.month(), date.day())
    }
}

fn group_scheduled_by_date(
    items: &[Arc<todos::entity::ItemModel>],
) -> Vec<(String, Vec<(usize, Arc<todos::entity::ItemModel>)>)> {
    let mut items_by_date: HashMap<String, Vec<(usize, Arc<todos::entity::ItemModel>)>> =
        HashMap::new();
    for (i, item) in items.iter().enumerate() {
        let date_key = item.due_date_ymd().unwrap_or_else(|| "无日期".to_string());
        items_by_date.entry(date_key).or_default().push((i, item.clone()));
    }
    let mut grouped: Vec<_> = items_by_date.into_iter().collect();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, items) in &mut grouped {
        items.sort_by(|a, b| {
            a.1.child_order
                .unwrap_or(i32::MAX)
                .cmp(&b.1.child_order.unwrap_or(i32::MAX))
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
    }
    grouped
}

impl BoardView for ScheduledBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for ScheduledBoard {
    fn icon() -> IconName {
        IconName::MonthSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xe6d4f2).into(), gpui::rgb(0x9b6ec0).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.scheduled_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "日程"
    }

    fn description() -> &'static str {
        ""
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScheduledBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for ScheduledBoard {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        // 在 render 开头处理待执行的刷新操作
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let board_count = ScheduledBoard::count(cx);
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let filter_date = self.filter_date.clone();
        let grouped_by_date = &self.grouped_by_date;
        let orange_color = gpui::hsla(38.0, 1.0, 0.53, 1.0);
        let date_picker = self.date_picker.clone();
        let calendar = self.calendar.clone();
        let has_filter = filter_date.is_some();

        v_flex()
            .id("scheduled-board")
            .track_focus(&self.base.focus_handle)
            .on_action(cx.listener(|this, _: &crate::MoveTaskUp, _, cx| {
                this.reorder_active(-1, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::MoveTaskDown, _, cx| {
                this.reorder_active(1, cx);
            }))
            .relative()
            .size_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.base.on_background_click(cx);
                }),
            )
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <ScheduledBoard as Board>::icon(),
                <ScheduledBoard as Board>::title(),
                <ScheduledBoard as Board>::description(),
                board_count,
                h_flex()
                    .gap(VisualHierarchy::spacing(2.0))
                    .child(DatePicker::new(&date_picker).cleanable(true).placeholder("按日期筛选"))
                    .when(has_filter, |this| {
                        this.child(
                            Button::new("clear-date-filter")
                                .small()
                                .ghost()
                                .label("全部")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.filter_date = None;
                                    cx.notify();
                                })),
                        )
                    })
                    .when(active_index.is_some(), |this| {
                        this.child(
                            Button::new("item-actions")
                                .small()
                                .ghost()
                                .compact()
                                .tooltip("任务操作")
                                .icon(IconName::CheckSquare)
                                .dropdown_menu({
                                    let view = view.clone();
                                    move |this, window, _cx| {
                                        let view = view.clone();
                                        this.item(
                                            PopupMenuItem::new("编辑任务")
                                                .icon(IconName::EditSymbolic)
                                                .on_click(window.listener_for(
                                                    &view,
                                                    |this, _, window, cx| {
                                                        this.show_item_dialog(
                                                            window, cx, true, None,
                                                        );
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("删除任务")
                                                .icon(IconName::UserTrashSymbolic)
                                                .on_click(window.listener_for(
                                                    &view,
                                                    |this, _, window, cx| {
                                                        this.show_item_delete_dialog(window, cx);
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                    }
                                }),
                        )
                    }),
            ))
            .child(crate::ui::views::boards::board_common::render_batch_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .child(
                        v_flex().w(px(280.)).px_2().pt_1().child(
                            Calendar::new(&calendar).first_day_of_week(chrono::Weekday::Mon),
                        ),
                    )
                    .child(
                        v_flex().flex_1().overflow_y_scrollbar().child(
                            v_flex()
                                .gap(VisualHierarchy::spacing(2.0))
                                .px_4()
                                .pt_1()
                                .pb(FAB_BOTTOM_PAD)
                                .when(item_rows.is_empty(), |this| {
                                    this.child(board_renderer::render_empty_placeholder(
                                        cx,
                                        ScheduledBoard::icon(),
                                        "添加一些任务",
                                        "设置日期后会按天分组显示",
                                    ))
                                })
                                .children(grouped_by_date.iter().filter_map(|(date, items)| {
                                    if items.is_empty() {
                                        return None;
                                    }
                                    if let Some(filter) = filter_date.as_deref()
                                        && date.as_str() != filter
                                    {
                                        return None;
                                    }

                                    let heading = format_schedule_heading(date, &today);
                                    let is_today = date.as_str() == today;
                                    let view_clone = view.clone();

                                    let title_color =
                                        if is_today { orange_color } else { cx.theme().foreground };

                                    Some(
                                        section_with_title(
                                            div()
                                                .text_base()
                                                .text_color(title_color)
                                                .child(heading),
                                        )
                                        .child(
                                            board_renderer::render_item_list(
                                                items,
                                                item_rows,
                                                active_index,
                                                active_border,
                                                view_clone,
                                                cx,
                                            ),
                                        ),
                                    )
                                })),
                        ),
                    ),
            )
            .child(crate::ui::views::boards::board_common::render_add_task_fab(
                "fab-add-scheduled",
                cx.listener(|this, _, window, cx| {
                    this.show_item_dialog(window, cx, false, None);
                }),
            ))
    }
}
