//! CompletedBoard - 已完成任务视图
//!
//! 显示已完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;

use crate::{
    BoardBase, VisualHierarchy,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, render_board_header, show_item_delete_dialog,
            show_item_unfinish_dialog, with_selected_item,
        },
        board_renderer,
        container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for CompletedBoard {}

pub struct CompletedBoard {
    base: BoardBase,
}

impl CompletedBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { base: BoardBase::new(window, cx) }
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

    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.base.apply_store_refresh(
            window,
            cx,
            crate::core::state::ChangeMask::affects_completed,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().completed_items_cached(cache)
            },
        );
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

    fn request_store_refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.base.request_refresh(cx);
    }
}

impl Board for CompletedBoard {
    fn icon() -> IconName {
        IconName::CheckRoundOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xffedd6).into(), gpui::rgb(0xe09a4c).into()]
    }

    fn count(cx: &mut App) -> usize {
        cx.global::<TodoStore>().completed_count()
    }

    fn title() -> String {
        t!("todo.board.completed").to_string()
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
        let board_count = CompletedBoard::count(cx);
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        v_flex()
            .id("completed-board")
            .track_focus(&self.base.focus_handle)
            .on_action(cx.listener(|this, _: &crate::MoveTaskUp, window, cx| {
                this.base.reorder_active(-1, window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::MoveTaskDown, window, cx| {
                this.base.reorder_active(1, window, cx);
            }))
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
                <CompletedBoard as Board>::icon(),
                <CompletedBoard as Board>::title(),
                <CompletedBoard as Board>::description(),
                board_count,
                h_flex().when(active_index.is_some(), |this| {
                    this.child(
                        Button::new("unfinish-item")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::Undo)
                            .tooltip(t!("todo.item.unfinish").to_string())
                            .on_click({
                                let view = view.clone();
                                move |_event, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.show_item_unfinish_dialog(window, cx);
                                        cx.notify();
                                    })
                                }
                            }),
                    )
                }),
            ))
            .child(crate::ui::views::boards::board_common::render_batch_bar(view.clone(), cx))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .px_4()
                        .py_1()
                        .gap(VisualHierarchy::spacing(2.0))
                        .when(item_rows.is_empty(), |this| {
                            this.child(board_renderer::render_empty_placeholder(
                                cx,
                                CompletedBoard::icon(),
                                t!("todo.empty.completed_title").to_string(),
                                t!("todo.empty.completed_hint").to_string(),
                            ))
                        })
                        .children(item_rows.iter().enumerate().map(move |(i, item_row)| {
                            let item_id = item_row.read(cx).item.id.clone();
                            let is_selected =
                                cx.global::<crate::core::state::ItemSelection>().contains(&item_id);
                            board_renderer::render_item_row(
                                i,
                                Some(item_row.clone()),
                                active_index == Some(i),
                                is_selected,
                                item_id,
                                active_border,
                                view.clone(),
                            )
                        })),
                ),
            )
    }
}
