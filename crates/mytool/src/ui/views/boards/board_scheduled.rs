//! ScheduledBoard - 计划任务视图
//!
//! 显示计划中任务，在其他时间去执行的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{cell::Cell, collections::HashMap, sync::Arc};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, VisualHierarchy, section_with_title,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FinishItemDialogStyle, render_board_header,
            show_finish_item_dialog, show_item_delete_dialog, show_pin_item_dialog,
            with_selected_item,
        },
        board_renderer,
        container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for ScheduledBoard {}

pub struct ScheduledBoard {
    base: BoardBase,
    /// 跟踪当前 item_rows 对应的 item id 列表，用于增量更新
    item_row_ids: Vec<String>,
    /// 脏标记：当 TodoStore 数据变化时设为 true，
    /// 在 render() 中执行实际的增量更新操作（需要 window 参数）
    pending_refresh: Cell<bool>,
    /// 延迟注册标记：避免在 new() 时立即注册全局观察者
    observer_registered: Cell<bool>,
}

impl ScheduledBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let base = BoardBase::new(window, cx);

        // 延迟注册：在首次 render 时通过 begin_pending_refresh 注册

        Self {
            base,
            item_row_ids: Vec::new(),
            pending_refresh: Cell::new(false),
            observer_registered: Cell::new(false),
        }
    }

    /// 在 render() 中执行实际的增量更新
    ///
    /// 只在 pending_refresh=true 时执行，避免每帧重复操作
    fn apply_pending_refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let has_store_data = if !self.pending_refresh.get() && self.base.item_rows.is_empty() {
            let cache = cx.global::<crate::core::state::QueryCache>();
            let items = cx.global::<TodoStore>().scheduled_items_cached(cache);
            !items.is_empty()
        } else {
            false
        };

        if !BoardBase::begin_pending_refresh(
            &self.observer_registered,
            &self.pending_refresh,
            &mut self.base._subscriptions,
            self.base.item_rows.is_empty(),
            has_store_data,
            cx,
            crate::core::state::ChangeMask::affects_scheduled,
            |this| &this.pending_refresh,
        ) {
            return;
        }

        let cache = cx.global::<crate::core::state::QueryCache>();
        let state_items = cx.global::<TodoStore>().scheduled_items_cached(cache);

        self.base.diff_update_item_rows(
            state_items.as_slice(),
            &mut self.item_row_ids,
            _window,
            cx,
        );
        self.base.update_items(state_items.as_slice());
        self.base.clamp_active_index();
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
        vec![gpui::rgb(0xdc8add).into(), gpui::rgb(0x9141ac).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.scheduled_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Scheduled"
    }

    fn description() -> &'static str {
        "计划中任务，在其他时间去执行的任务"
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
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let cache = cx.global::<crate::core::state::QueryCache>();
        let all_scheduled = cx.global::<TodoStore>().scheduled_items_cached(cache);

        let mut items_by_date: HashMap<String, Vec<(usize, Arc<todos::entity::ItemModel>)>> =
            HashMap::new();

        for (i, item) in all_scheduled.iter().enumerate() {
            let date_key = item
                .due_date()
                .and_then(|d| {
                    let date_str = &d.date;
                    if date_str.contains('T') {
                        date_str.split('T').next().map(String::from)
                    } else {
                        date_str.split(' ').next().map(String::from)
                    }
                })
                .unwrap_or_else(|| "无日期".to_string());
            items_by_date.entry(date_key).or_default().push((i, item.clone()));
        }

        let mut sorted_dates: Vec<_> = items_by_date.into_keys().collect();
        sorted_dates.sort();

        let orange_color = gpui::hsla(38.0, 1.0, 0.53, 1.0);

        v_flex()
            .track_focus(&self.base.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <ScheduledBoard as Board>::icon(),
                <ScheduledBoard as Board>::title(),
                <ScheduledBoard as Board>::description(),
                Button::new("add-section")
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::PlusLargeSymbolic)
                    .label("Add Section")
                    .on_click({
                        let view = view.clone();
                        move |_event, window, cx| {
                            view.update(cx, |this, cx| {
                                this.show_section_dialog(window, cx, None, false);
                                cx.notify();
                            })
                        }
                    }),
            ))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .children(sorted_dates.into_iter().filter_map(|date| {
                            let items = all_scheduled
                                .iter()
                                .enumerate()
                                .filter(|(_, item)| {
                                    let item_date = item
                                        .due_date()
                                        .and_then(|d| {
                                            let date_str = &d.date;
                                            if date_str.contains('T') {
                                                date_str.split('T').next().map(String::from)
                                            } else {
                                                date_str.split(' ').next().map(String::from)
                                            }
                                        })
                                        .unwrap_or_else(|| "无日期".to_string());
                                    item_date == date
                                })
                                .map(|(i, item)| (i, item.clone()))
                                .collect::<Vec<_>>();

                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();
                            let is_today = date == today;

                            let title_color =
                                if is_today { orange_color } else { cx.theme().foreground };

                            Some(
                                section_with_title(div().flex().items_center().gap_2().child(
                                    div().text_base().text_color(title_color).child(date.clone()),
                                ))
                                .sub_title(
                                    h_flex().gap_1().child(
                                        Button::new(format!("more-date-{}", date))
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::EllipsisVertical)
                                            .dropdown_menu({
                                                let view = view_clone.clone();
                                                move |this, window, _cx| {
                                                    this.item(
                                                        PopupMenuItem::new("Show Completed Tasks")
                                                            .on_click(window.listener_for(
                                                                &view,
                                                                |_this, _, _window, cx| {
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    )
                                                }
                                            }),
                                    ),
                                )
                                .child(
                                    board_renderer::render_item_list(
                                        &items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ),
                                ),
                            )
                        })),
                ),
            )
    }
}
