//! ScheduledBoard - 计划任务视图
//!
//! 显示计划中任务，在其他时间去执行的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{collections::HashMap, sync::Arc};

use chrono::{Datelike, NaiveDate};
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div, prelude::FluentBuilder,
    px,
};
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    calendar::{Calendar, CalendarEvent, CalendarState, Date},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;

use crate::{
    BoardBase, VisualHierarchy, section_with_title,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FAB_BOTTOM_PAD, FinishItemDialogStyle, render_board_header,
            show_finish_item_dialog, show_item_delete_dialog, show_pin_item_dialog, weekday_short,
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
    /// 按日历日分组的缓存（在 refresh 时构建，render 只读）。`None` 表示无日期。
    grouped_by_date: Vec<(Option<NaiveDate>, Vec<(usize, Arc<todos::entity::ItemModel>)>)>,
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

    pub fn reorder_by_item_id(
        &mut self,
        item_id: &str,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.reorder_by_item_id(item_id, delta, window, cx);
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let calendar = cx.new(|cx| CalendarState::new(window, cx));
        let _date_subscription =
            cx.subscribe_in(&date_picker, window, |this, _, event, window, cx| {
                let DatePickerEvent::Change(date) = event;
                this.filter_date = date.format("%Y-%m-%d").map(|s| s.to_string());
                if let Some(ymd) = this.filter_ymd() {
                    this.calendar.update(cx, |cal, cx| {
                        cal.set_date(ymd, window, cx);
                    });
                }
                cx.notify();
            });
        let _calendar_subscription =
            cx.subscribe_in(&calendar, window, |this, _, event, window, cx| {
                let CalendarEvent::Selected(date) = event;
                this.filter_date = date.format("%Y-%m-%d").map(|s| s.to_string());
                if let Some(ymd) = this.filter_ymd() {
                    this.date_picker.update(cx, |picker, cx| {
                        picker.set_date(ymd, window, cx);
                    });
                }
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

    fn filter_ymd(&self) -> Option<NaiveDate> {
        self.filter_date.as_deref().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    fn clear_date_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_date = None;
        self.date_picker.update(cx, |picker, cx| {
            picker.set_date(Date::Single(None), window, cx);
        });
        cx.notify();
    }

    fn select_filter_date(&mut self, ymd: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_date = Some(ymd.format("%Y-%m-%d").to_string());
        self.date_picker.update(cx, |picker, cx| {
            picker.set_date(ymd, window, cx);
        });
        self.calendar.update(cx, |cal, cx| {
            cal.set_date(ymd, window, cx);
        });
        cx.notify();
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
        self.base.request_refresh(cx);
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
            if let Some(filter) = self.filter_ymd() {
                let still_has = self
                    .grouped_by_date
                    .iter()
                    .any(|(date, items)| *date == Some(filter) && !items.is_empty());
                if !still_has {
                    self.clear_date_filter(window, cx);
                }
            }
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

fn format_schedule_heading(date: Option<NaiveDate>, today: NaiveDate) -> String {
    let Some(date) = date else {
        return t!("todo.date.none").to_string();
    };
    let weekday = weekday_short(date.weekday().num_days_from_sunday());

    if date == today {
        t!("todo.date.today_weekday", weekday => weekday.as_str()).to_string()
    } else if today.succ_opt() == Some(date) {
        t!("todo.date.tomorrow_weekday", weekday => weekday.as_str()).to_string()
    } else if today.pred_opt() == Some(date) {
        t!("todo.date.yesterday_weekday", weekday => weekday.as_str()).to_string()
    } else if date.year() == today.year() {
        t!(
            "todo.date.weekday_md",
            weekday => weekday.as_str(),
            month => date.month(),
            day => date.day()
        )
        .to_string()
    } else {
        t!(
            "todo.date.ymd",
            year => date.year(),
            month => date.month(),
            day => date.day()
        )
        .to_string()
    }
}

fn group_scheduled_by_date(
    items: &[Arc<todos::entity::ItemModel>],
) -> Vec<(Option<NaiveDate>, Vec<(usize, Arc<todos::entity::ItemModel>)>)> {
    let mut items_by_date: HashMap<Option<NaiveDate>, Vec<(usize, Arc<todos::entity::ItemModel>)>> =
        HashMap::new();
    for (i, item) in items.iter().enumerate() {
        items_by_date.entry(item.due_date_naive()).or_default().push((i, item.clone()));
    }
    let mut grouped: Vec<_> = items_by_date.into_iter().collect();
    grouped.sort_by(|a, b| match (a.0, b.0) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
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

    fn request_store_refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.base.request_refresh(cx);
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

    fn title() -> String {
        t!("todo.board.scheduled").to_string()
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

        let today = chrono::Local::now().date_naive();
        let filter_date = self.filter_ymd();
        let grouped_by_date = &self.grouped_by_date;
        let orange_color = gpui::hsla(38.0, 1.0, 0.53, 1.0);
        let date_picker = self.date_picker.clone();
        let calendar = self.calendar.clone();
        let has_filter = self.filter_date.is_some();

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
                    .child(
                        DatePicker::new(&date_picker)
                            .cleanable(true)
                            .placeholder(t!("todo.date.filter").to_string()),
                    )
                    .when(has_filter, |this| {
                        this.child(
                            Button::new("clear-date-filter")
                                .small()
                                .ghost()
                                .label(t!("todo.date.all").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.clear_date_filter(window, cx);
                                })),
                        )
                    })
                    .when(active_index.is_some(), |this| {
                        this.child(
                            Button::new("item-actions")
                                .small()
                                .ghost()
                                .compact()
                                .tooltip(t!("todo.item.actions").to_string())
                                .icon(IconName::CheckSquare)
                                .dropdown_menu({
                                    let view = view.clone();
                                    move |this, window, _cx| {
                                        let view = view.clone();
                                        this.item(
                                            PopupMenuItem::new(t!("todo.item.edit").to_string())
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
                                            PopupMenuItem::new(t!("todo.item.delete").to_string())
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
            .child(crate::ui::views::boards::board_common::render_batch_bar(view.clone(), cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .child(
                        v_flex()
                            .w(px(280.))
                            .px_2()
                            .pt_1()
                            .gap_2()
                            .child(Calendar::new(&calendar).first_day_of_week(chrono::Weekday::Mon))
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(t!("todo.date.with_tasks").to_string()),
                                    )
                                    .child(h_flex().gap_1().flex_wrap().children(
                                        grouped_by_date.iter().filter_map(|(date, items)| {
                                            let ymd = (*date)?;
                                            if items.is_empty() {
                                                return None;
                                            }
                                            let selected = filter_date == Some(ymd);
                                            let label = t!(
                                                "todo.date.day_count",
                                                day => ymd.day(),
                                                count => items.len()
                                            )
                                            .to_string();
                                            Some(
                                                Button::new((
                                                    "sched-day",
                                                    ymd.num_days_from_ce() as usize,
                                                ))
                                                .small()
                                                .when(selected, |this| this.primary())
                                                .when(!selected, |this| this.ghost())
                                                .label(label)
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.select_filter_date(ymd, window, cx);
                                                    },
                                                )),
                                            )
                                        }),
                                    )),
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
                                        t!("todo.empty.add_tasks").to_string(),
                                        t!("todo.empty.scheduled_hint").to_string(),
                                    ))
                                })
                                .children(grouped_by_date.iter().filter_map(|(date, items)| {
                                    if items.is_empty() {
                                        return None;
                                    }
                                    if let Some(filter) = filter_date
                                        && *date != Some(filter)
                                    {
                                        return None;
                                    }

                                    let heading = format_schedule_heading(*date, today);
                                    let is_today = *date == Some(today);
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use todos::DueDate;

    use super::group_scheduled_by_date;

    fn item_on(id: &str, date: Option<&str>) -> Arc<todos::entity::ItemModel> {
        let mut model = todos::entity::ItemModel::default();
        model.id = id.into();
        if let Some(date) = date {
            let mut due = DueDate::default();
            due.date = date.into();
            model.set_due_date(Some(due));
        }
        Arc::new(model)
    }

    #[test]
    fn scheduled_groups_sort_dates_and_keep_undated_last() {
        let grouped = group_scheduled_by_date(&[
            item_on("later", Some("2026-03-02")),
            item_on("none", None),
            item_on("earlier", Some("2026-03-01")),
        ]);
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].0, NaiveDate::from_ymd_opt(2026, 3, 1));
        assert_eq!(grouped[0].1[0].1.id, "earlier");
        assert_eq!(grouped[1].0, NaiveDate::from_ymd_opt(2026, 3, 2));
        assert_eq!(grouped[2].0, None);
        assert_eq!(grouped[2].1[0].1.id, "none");
    }
}
