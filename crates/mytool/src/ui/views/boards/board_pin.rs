//! PinBoard - 置顶任务视图
//!
//! 显示重点关注的置顶任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Window, prelude::FluentBuilder,
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
    BoardBase, VisualHierarchy, section,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FinishItemDialogStyle, render_board_header,
            show_finish_item_dialog, show_item_delete_dialog, show_pin_item_dialog,
            with_selected_item,
        },
        board_renderer::{self, SectionBlockOptions},
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
        vec![gpui::rgb(0xf66151).into(), gpui::rgb(0xed333b).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.pinned_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Pinboard"
    }

    fn description() -> &'static str {
        "重点关注任务"
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
        let sections = &cx.global::<TodoStore>().sections;
        let pinned_items = &self.base.pinned_items;
        let no_section_items = &self.base.no_section_items;
        let section_items_map = &self.base.section_items_map;
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        v_flex()
            .track_focus(&self.base.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <PinBoard as Board>::icon(),
                <PinBoard as Board>::title(),
                <PinBoard as Board>::description(),
                Button::new("unpin-item")
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::PinSymbolic)
                    .on_click({
                        let view = view.clone();
                        move |_event, window, cx| {
                            view.update(cx, |this, cx| {
                                this.show_unpin_item_dialog(window, cx);
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
                        .when(!pinned_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("Pinned")
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new("more-pinned")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::EllipsisVertical)
                                                .dropdown_menu({
                                                    let view = view_clone.clone();
                                                    move |this, window, _cx| {
                                                        this.item(
                                                            PopupMenuItem::new(
                                                                "Show Completed Tasks",
                                                            )
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
                                    .child(board_renderer::render_item_list(
                                        &pinned_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    )),
                            )
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
                                SectionBlockOptions { show_inline_edit_delete: false },
                            ))
                        })),
                ),
            )
    }
}
