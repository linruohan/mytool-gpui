//! TodayBoard - 今日任务视图
//!
//! 显示今天需要完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, InteractiveElement, MouseButton,
    ParentElement, Render, Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;

use crate::{
    BoardBase, ScheduleButtonEvent, ScheduleButtonState, VisualHierarchy,
    core::actions::batch::batch_update_items,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FAB_BOTTOM_PAD, render_board_header, show_item_delete_dialog,
            show_schedule_popover, with_selected_item,
        },
        board_renderer,
        container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for TodayBoard {}

pub struct TodayBoard {
    base: BoardBase,
    /// Past Due 分组的 ScheduleButton 状态
    past_due_schedule_button: Entity<ScheduleButtonState>,
    /// ScheduleButton 事件订阅
    _schedule_subscription: Subscription,
}

impl TodayBoard {
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
        let mut base = BoardBase::new(window, cx);
        base.is_today_board = true;

        let past_due_schedule_button = cx.new(|cx| ScheduleButtonState::new(window, cx));

        let schedule_subscription =
            cx.subscribe_in(&past_due_schedule_button, window, |this, _, event, window, cx| {
                let due_date = this.past_due_schedule_button.read(cx).due_date.clone();
                if this.base.past_due_items.is_empty() {
                    return;
                }

                let mut updated_items: Vec<_> =
                    this.base.past_due_items.iter().map(|(_, item)| item.clone()).collect();

                match event {
                    ScheduleButtonEvent::DateSelected(_) | ScheduleButtonEvent::TimeSelected(_) => {
                        for item in &mut updated_items {
                            Arc::make_mut(item).set_due_date(Some(due_date.clone()));
                        }
                        let count = updated_items.len();
                        batch_update_items(updated_items, cx);
                        window.push_notification(
                            t!("todo.today.rescheduled_n", count => count).to_string(),
                            cx,
                        );
                    },
                    ScheduleButtonEvent::Cleared => {
                        for item in &mut updated_items {
                            Arc::make_mut(item).set_due_date(None);
                        }
                        batch_update_items(updated_items, cx);
                    },
                }
                this.base.request_refresh(cx);
            });

        // 延迟注册：在首次 render 时通过 begin_pending_refresh 注册

        Self { base, past_due_schedule_button, _schedule_subscription: schedule_subscription }
    }

    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.base.apply_store_refresh(
            window,
            cx,
            crate::core::state::ChangeMask::affects_today,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().today_items_cached(cache)
            },
        );
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }
}

crate::impl_board_section_forwards!(TodayBoard);
crate::impl_board_section_actions!(TodayBoard);

impl BoardView for TodayBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }

    fn request_store_refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.base.request_refresh(cx);
    }
}

impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<gpui::Hsla> {
        vec![gpui::rgb(0xd4f3de).into(), gpui::rgb(0x4caf78).into()]
    }

    fn count(cx: &mut gpui::App) -> usize {
        cx.global::<TodoStore>().today_count()
    }

    fn title() -> String {
        t!("todo.board.today").to_string()
    }

    fn description() -> &'static str {
        ""
    }

    fn zoomable() -> Option<gpui_component::dock::PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut gpui::App) -> Entity<impl gpui::Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TodayBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for TodayBoard {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        // 在 render 开头处理待执行的刷新操作
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let board_count = TodayBoard::count(cx);
        let sections = cx.global::<TodoStore>().sections_in_order();
        let pinned_items = &self.base.pinned_items;
        let past_due_items = &self.base.past_due_items;
        let due_today_items = &self.base.due_today_items;
        let no_section_items = &self.base.no_section_items;
        let section_items_map = &self.base.section_items_map;
        let has_named_sections = section_items_map.values().any(|items| !items.is_empty());
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;
        let past_due_schedule_button = self.past_due_schedule_button.clone();

        let date_label = {
            use chrono::Datelike;
            let now = chrono::Local::now();
            let weekday = crate::ui::views::boards::board_common::weekday_short(
                now.weekday().num_days_from_sunday(),
            );
            t!(
                "todo.date.weekday_md",
                weekday => weekday.as_str(),
                month => now.month(),
                day => now.day()
            )
            .to_string()
        };

        v_flex()
            .id("today-board")
            .track_focus(&self.base.focus_handle)
            .on_action(cx.listener(|this, _: &crate::MoveTaskUp, window, cx| {
                this.base.reorder_active(-1, window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::MoveTaskDown, window, cx| {
                this.base.reorder_active(1, window, cx);
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
                <TodayBoard as Board>::icon(),
                <TodayBoard as Board>::title(),
                date_label,
                board_count,
                h_flex().gap(VisualHierarchy::spacing(2.0)).when(active_index.is_some(), |this| {
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
                                                    this.show_item_dialog(window, cx, true, None);
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
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(2.0))
                        .px_4()
                        .pt_1()
                        .pb(FAB_BOTTOM_PAD)
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(board_renderer::render_simple_group_block(
                                &t!("todo.group.pinned"),
                                &pinned_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                true,
                                cx,
                            ))
                        })
                        .when(item_rows.is_empty(), |this| {
                            this.child(board_renderer::render_empty_placeholder(
                                cx,
                                TodayBoard::icon(),
                                t!("todo.empty.add_tasks").to_string(),
                                t!("todo.empty.add_hint").to_string(),
                            ))
                        })
                        .when(!past_due_items.is_empty(), |this| {
                            this.child(board_renderer::render_group_with_schedule_button(
                                &t!("todo.group.overdue"),
                                &past_due_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                &past_due_schedule_button,
                                cx,
                            ))
                        })
                        .when(!due_today_items.is_empty(), |this| {
                            this.child(board_renderer::render_item_list(
                                &due_today_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                cx,
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            if has_named_sections {
                                this.child(board_renderer::render_no_section_block(
                                    &no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    true,
                                    cx,
                                ))
                            } else {
                                this.child(board_renderer::render_item_list(
                                    &no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    cx,
                                ))
                            }
                        })
                        .children(sections.iter().filter_map(|sec| {
                            let items = section_items_map.get(&sec.id)?;
                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();
                            let section_id = sec.id.clone();
                            let section_name = sec.name.clone();

                            let schedule_button =
                                Button::new(format!("schedule-section-{}", section_id))
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Calendar)
                                    .label(t!("todo.section.schedule").to_string())
                                    .on_click({
                                        let section_id = section_id.clone();
                                        let view = view.clone();
                                        move |_, window, cx| {
                                            show_schedule_popover(
                                                window,
                                                cx,
                                                section_id.clone(),
                                                view.clone(),
                                            );
                                        }
                                    });

                            Some(board_renderer::render_section_block_with_leading(
                                section_name,
                                section_id,
                                items,
                                item_rows,
                                active_index,
                                active_border,
                                view_clone,
                                schedule_button,
                                cx,
                            ))
                        })),
                ),
            )
            .child(crate::ui::views::boards::board_common::render_add_task_fab(
                "fab-add-today",
                cx.listener(|this, _, window, cx| {
                    this.show_item_dialog(window, cx, false, None);
                }),
            ))
    }
}
