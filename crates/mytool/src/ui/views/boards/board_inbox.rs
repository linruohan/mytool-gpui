//! InboxBoard - 收件箱视图
//!
//! 显示所有未完成且无项目的任务。
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
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;

use crate::{
    BoardBase, VisualHierarchy, board_section,
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

impl EventEmitter<BoardItemClickEvent> for InboxBoard {}

pub struct InboxBoard {
    base: BoardBase,
}

impl InboxBoard {
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
            crate::core::state::ChangeMask::affects_inbox,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().inbox_items_cached(cache)
            },
        );
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_finish_item_dialog(window, cx, item, FinishItemDialogStyle::Inbox);
        });
    }

    pub fn show_pin_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_pin_item_dialog(window, cx, item);
        });
    }
}

crate::impl_board_section_forwards!(InboxBoard);
crate::impl_board_section_actions!(InboxBoard);

impl BoardView for InboxBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for InboxBoard {
    fn icon() -> IconName {
        IconName::MailboxSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xd4e6fb).into(), gpui::rgb(0x5b9ae8).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.inbox_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "收件箱"
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

impl Focusable for InboxBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for InboxBoard {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let board_count = InboxBoard::count(cx);
        let inbox_sections: Vec<_> = cx
            .global::<TodoStore>()
            .sections
            .iter()
            .filter(|s| {
                !s.is_archived
                    && !s.is_deleted
                    && !s.hidded
                    && s.project_id.as_deref().unwrap_or("").is_empty()
            })
            .cloned()
            .collect();
        let has_inbox_sections = !inbox_sections.is_empty();
        let pinned_items = &self.base.pinned_items;
        let no_section_items = &self.base.no_section_items;
        let section_items_map = &self.base.section_items_map;
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        v_flex()
            .id("inbox-board")
            .track_focus(&self.base.focus_handle)
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
                <InboxBoard as Board>::icon(),
                <InboxBoard as Board>::title(),
                <InboxBoard as Board>::description(),
                board_count,
                h_flex()
                    .gap(VisualHierarchy::spacing(2.0))
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
                    })
                    .child(
                        Button::new("add-action")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PlusLargeSymbolic)
                            .tooltip("新建分区")
                            .on_click({
                                let view = view.clone();
                                move |_event, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.show_section_dialog(window, cx, None, false);
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            ))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(2.0))
                        .px_4()
                        .pt_1()
                        .pb(FAB_BOTTOM_PAD)
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(board_section("置顶").child(
                                board_renderer::render_item_list(
                                    &pinned_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                ),
                            ))
                        })
                        .when(item_rows.is_empty(), |this| {
                            this.child(board_renderer::render_empty_placeholder(
                                cx,
                                InboxBoard::icon(),
                                "添加一些任务",
                                "点击右下角 + 创建新任务",
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            if has_inbox_sections {
                                this.child(board_renderer::render_no_section_block(
                                    &no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    false,
                                ))
                            } else {
                                this.child(board_renderer::render_item_list(
                                    &no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                ))
                            }
                        })
                        .children(inbox_sections.iter().map(|sec| {
                            let items =
                                section_items_map.get(&sec.id).map(|v| v.as_slice()).unwrap_or(&[]);
                            board_renderer::render_section_block(
                                sec.name.clone(),
                                sec.id.clone(),
                                items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                            )
                        })),
                ),
            )
            .child(crate::ui::views::boards::board_common::render_add_task_fab(
                "fab-add-inbox",
                cx.listener(|this, _, window, cx| {
                    this.show_item_dialog(window, cx, false, None);
                }),
            ))
    }
}
