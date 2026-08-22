//! TodayBoard - 今日任务视图
//!
//! 显示今天需要完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::sync::Arc;

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
    /// Past Due 分组的 ScheduleButton 状态
    past_due_schedule_button: Entity<ScheduleButtonState>,
    /// ScheduleButton 事件订阅
    _schedule_subscription: Subscription,
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
                        window.push_notification(format!("Rescheduled {count} items"), cx);
                    },
                    ScheduleButtonEvent::Cleared => {
                        for item in &mut updated_items {
                            Arc::make_mut(item).set_due_date(None);
                        }
                        batch_update_items(updated_items, cx);
                    },
                }
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
}

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
        let pinned_items = &self.base.pinned_items;
        let past_due_items = &self.base.past_due_items;
        let due_today_items = &self.base.due_today_items;
        let no_section_items = &self.base.no_section_items;
        let section_items_map = &self.base.section_items_map;
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
