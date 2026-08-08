//! TodayBoard - 今日任务视图
//!
//! 显示今天需要完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{cell::Cell, sync::Arc};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, InteractiveElement, ParentElement,
    Render, Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, ScheduleButtonEvent, ScheduleButtonState, VisualHierarchy,
    core::actions::batch::batch_update_items,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, render_board_header, show_item_delete_dialog,
            show_schedule_popover, with_selected_item,
        },
        board_renderer,
        container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for TodayBoard {}

pub struct TodayBoard {
    base: BoardBase,
    /// 跟踪当前 item_rows 对应的 item id 列表，用于增量更新
    item_row_ids: Vec<String>,
    /// Past Due 分组的 ScheduleButton 状态
    past_due_schedule_button: Entity<ScheduleButtonState>,
    /// ScheduleButton 事件订阅
    _schedule_subscription: Subscription,
    /// 脏标记：当 TodoStore 数据变化时设为 true，
    /// 在 render() 中执行实际的增量更新操作（需要 window 参数）
    pending_refresh: Cell<bool>,
    /// 延迟注册标记：避免在 new() 时立即注册全局观察者
    observer_registered: Cell<bool>,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);
        base.is_today_board = true;

        let past_due_schedule_button = cx.new(|cx| ScheduleButtonState::new(window, cx));

        let schedule_subscription =
            cx.subscribe_in(&past_due_schedule_button, window, |this, _, event, window, cx| {
                let due_date = this.past_due_schedule_button.read(cx).due_date.clone();

                let store = cx.global::<TodoStore>();
                let past_due_items: Vec<Arc<todos::entity::ItemModel>> = store
                    .all_items
                    .iter()
                    .filter(|item| !item.checked && item.is_past_due())
                    .cloned()
                    .collect();

                if past_due_items.is_empty() {
                    return;
                }

                match event {
                    ScheduleButtonEvent::DateSelected(_) | ScheduleButtonEvent::TimeSelected(_) => {
                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(Some(due_date.clone()));
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        let count = updated_items.len();
                        batch_update_items(updated_items, cx);
                        window.push_notification(format!("Rescheduled {} items", count), cx);
                    },
                    ScheduleButtonEvent::Cleared => {
                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(None);
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        batch_update_items(updated_items, cx);
                    },
                }
            });

        // 延迟注册：在首次 render 时通过 begin_pending_refresh 注册

        Self {
            base,
            item_row_ids: Vec::new(),
            past_due_schedule_button,
            _schedule_subscription: schedule_subscription,
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
            let items = cx.global::<TodoStore>().today_items_cached(cache);
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
            crate::core::state::ChangeMask::affects_today,
            |this| &this.pending_refresh,
        ) {
            return;
        }

        let cache = cx.global::<crate::core::state::QueryCache>();
        let state_items = cx.global::<TodoStore>().today_items_cached(cache);

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

    pub fn duplicate_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        self.base.duplicate_section(window, cx, section_id);
    }

    pub fn archive_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        self.base.archive_section(window, cx, section_id);
    }
}

impl BoardView for TodayBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

crate::impl_board_section_actions!(TodayBoard);

impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<gpui::Hsla> {
        vec![gpui::rgb(0x33d17a).into(), gpui::rgb(0x33d17a).into()]
    }

    fn count(cx: &mut gpui::App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.today_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "今天需要完成的任务"
    }

    fn zoomable() -> Option<gpui_component::dock::PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut gpui::App) -> Entity<impl gpui::Render> {
        Self::view(window, cx)
    }
}

/// 创建头部按钮的辅助函数
fn create_header_button<F>(
    id: String,
    icon: IconName,
    label: Option<&'static str>,
    view: Entity<TodayBoard>,
    action: F,
) -> impl gpui::IntoElement
where
    F: Fn(&mut TodayBoard, &mut Window, &mut Context<TodayBoard>) + 'static + Clone,
{
    let mut button = Button::new(id).small().ghost().compact().icon(icon);

    if let Some(label_text) = label {
        button = button.label(label_text);
    }

    button.on_click({
        let view = view.clone();
        move |_event, window, cx| {
            view.update(cx, |this, cx| {
                action(this, window, cx);
                cx.notify();
            })
        }
    })
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
        let sections = &cx.global::<TodoStore>().sections;
        let pinned_items = self.base.pinned_items.clone();
        let past_due_items = self.base.past_due_items.clone();
        let due_today_items = self.base.due_today_items.clone();
        let no_section_items = self.base.no_section_items.clone();
        let section_items_map = self.base.section_items_map.clone();
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;
        let past_due_schedule_button = self.past_due_schedule_button.clone();

        v_flex()
            .track_focus(&self.base.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <TodayBoard as Board>::icon(),
                <TodayBoard as Board>::title(),
                <TodayBoard as Board>::description(),
                h_flex()
                    .gap(VisualHierarchy::spacing(2.0))
                    .child(
                        Button::new("item-actions")
                            .small()
                            .ghost()
                            .compact()
                            .tooltip("Item Operation")
                            .icon(IconName::CheckSquare)
                            .dropdown_menu({
                                let view = view.clone();
                                move |this, window, _cx| {
                                    let view = view.clone();
                                    this.item(
                                        PopupMenuItem::new("Add Item")
                                            .icon(IconName::PlusLargeSymbolic)
                                            .on_click(window.listener_for(
                                                &view,
                                                |this, _, window, cx| {
                                                    this.show_item_dialog(window, cx, false, None);
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                    .separator()
                                    .item(
                                        PopupMenuItem::new("Edit Item")
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
                                        PopupMenuItem::new("Delete Item")
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
                    .child(create_header_button(
                        "section-actions".to_string(),
                        IconName::PlusLargeSymbolic,
                        Some("Add Section"),
                        view.clone(),
                        |this, window, cx| this.show_section_dialog(window, cx, None, false),
                    )),
            ))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(board_renderer::render_simple_group_block(
                                "Pinned",
                                &pinned_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                true,
                            ))
                        })
                        .when(!past_due_items.is_empty(), |this| {
                            this.child(board_renderer::render_group_with_schedule_button(
                                "Past Due",
                                &past_due_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                &past_due_schedule_button,
                            ))
                        })
                        .when(!due_today_items.is_empty(), |this| {
                            this.child(board_renderer::render_simple_group_block(
                                "Today",
                                &due_today_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                true,
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            this.child(board_renderer::render_no_section_block(
                                &no_section_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                true,
                            ))
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
                                    .label("Schedule")
                                    .on_click({
                                        let section_id = section_id.clone();
                                        move |_, window, cx| {
                                            show_schedule_popover(window, cx, section_id.clone());
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
                            ))
                        })),
                ),
            )
    }
}
