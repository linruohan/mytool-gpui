//! PinBoard - 置顶任务视图
//!
//! 显示重点关注的置顶任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Window,
    prelude::FluentBuilder,
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

use crate::{
    BoardBase, VisualHierarchy,
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
    },
};

impl EventEmitter<BoardItemClickEvent> for PinBoard {}

pub struct PinBoard {
    base: BoardBase,
}

impl PinBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { base: BoardBase::new(window, cx) }
    }

    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.base.apply_store_refresh(
            window,
            cx,
            crate::core::state::ChangeMask::affects_pinned,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().pinned_items_cached(cache)
            },
        );
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }

    pub fn show_unpin_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_pin_item_dialog(window, cx, item);
        });
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_finish_item_dialog(window, cx, item, FinishItemDialogStyle::Standard);
        });
    }
}

crate::impl_board_section_forwards!(PinBoard);
crate::impl_board_section_actions!(PinBoard);

impl BoardView for PinBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for PinBoard {
    fn icon() -> IconName {
        IconName::PinSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xf8e0dc).into(), gpui::rgb(0xe07070).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.pinned_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "置顶"
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

impl Focusable for PinBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for PinBoard {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        // 在 render 开头处理待执行的刷新操作
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let board_count = PinBoard::count(cx);
        let sections = &cx.global::<TodoStore>().sections;
        let pinned_items = &self.base.pinned_items;
        let no_section_items = &self.base.no_section_items;
        let section_items_map = &self.base.section_items_map;
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        v_flex()
            .id("pin-board")
            .track_focus(&self.base.focus_handle)
            .relative()
            .size_full()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.base.collapse_open_rows(cx);
            }))
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <PinBoard as Board>::icon(),
                <PinBoard as Board>::title(),
                <PinBoard as Board>::description(),
                board_count,
                h_flex().when(active_index.is_some(), |this| {
                    this.child(
                        Button::new("unpin-item")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PinSymbolic)
                            .tooltip("取消置顶")
                            .on_click({
                                let view = view.clone();
                                move |_event, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.show_unpin_item_dialog(window, cx);
                                        cx.notify();
                                    })
                                }
                            }),
                    )
                }),
            ))
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
                                PinBoard::icon(),
                                "没有置顶任务",
                                "把重要任务钉在这里，方便随时看到",
                            ))
                        })
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(board_renderer::render_item_list(
                                &pinned_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
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

                            Some(board_renderer::render_section_block(
                                sec.name.clone(),
                                sec.id.clone(),
                                items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                            ))
                        })),
                ),
            )
            .child(crate::ui::views::boards::board_common::render_add_task_fab(
                "fab-add-pin",
                cx.listener(|this, _, window, cx| {
                    this.show_item_dialog(window, cx, false, None);
                }),
            ))
    }
}
