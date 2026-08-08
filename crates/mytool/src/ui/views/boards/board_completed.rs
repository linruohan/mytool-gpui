//! CompletedBoard - 已完成任务视图
//!
//! 显示已完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::cell::Cell;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Window,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, VisualHierarchy,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, render_board_header, show_item_delete_dialog,
            show_item_unfinish_dialog, with_selected_item,
        },
        board_renderer, container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for CompletedBoard {}

pub struct CompletedBoard {
    base: BoardBase,
    /// 跟踪当前 item_rows 对应的 item id 列表，用于增量更新
    item_row_ids: Vec<String>,
    /// 脏标记：当 TodoStore 数据变化时设为 true，
    /// 在 render() 中执行实际的增量更新操作（需要 window 参数）
    pending_refresh: Cell<bool>,
    /// 延迟注册标记：避免在 new() 时立即注册全局观察者
    observer_registered: Cell<bool>,
}

impl CompletedBoard {
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
            let items = cx.global::<TodoStore>().completed_items_cached(cache);
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
            crate::core::state::ChangeMask::affects_completed,
            |this| &this.pending_refresh,
        ) {
            return;
        }

        let cache = cx.global::<crate::core::state::QueryCache>();
        let state_items = cx.global::<TodoStore>().completed_items_cached(cache);

        self.base.diff_update_item_rows(state_items.as_slice(), &mut self.item_row_ids, _window, cx);
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

    pub fn show_item_unfinish_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_unfinish_dialog(window, cx, item);
        });
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }
}

impl BoardView for CompletedBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for CompletedBoard {
    fn icon() -> IconName {
        IconName::CheckRoundOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xffbe6f).into(), gpui::rgb(0xff7800).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.completed_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Completed"
    }

    fn description() -> &'static str {
        "已完成任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for CompletedBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for CompletedBoard {
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

        v_flex()
            .track_focus(&self.base.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
            .child(
                render_board_header(
                    cx,
                    <CompletedBoard as Board>::icon(),
                    <CompletedBoard as Board>::title(),
                    <CompletedBoard as Board>::description(),
                    Button::new("unfinish-item")
                        .small()
                        .ghost()
                        .compact()
                        .icon(IconName::Undo)
                        .on_click({
                            let view = view.clone();
                            move |_event, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.show_item_unfinish_dialog(window, cx);
                                    cx.notify();
                                })
                            }
                        }),
                ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p(VisualHierarchy::spacing(3.0))
                    .gap(VisualHierarchy::spacing(2.0))
                    .children(item_rows.iter().enumerate().map(move |(i, item_row)| {
                        let is_active = active_index == Some(i);
                        board_renderer::render_item_row(
                            i,
                            Some(item_row.clone()),
                            is_active,
                            active_border,
                            view.clone(),
                        )
                    })),
            )
    }
}
