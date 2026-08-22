//! InboxBoard - 收件箱视图
//!
//! 显示所有未完成且无项目的任务。
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
        vec![gpui::rgb(0x99c1f1).into(), gpui::rgb(0x3584e4).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.inbox_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Inbox"
    }

    fn description() -> &'static str {
        "未完成的无项目任务，去掉今天"
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
                <InboxBoard as Board>::icon(),
                <InboxBoard as Board>::title(),
                <InboxBoard as Board>::description(),
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
                    .child(
                        Button::new("add-action")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PlusLargeSymbolic)
                            .label("Section")
                            .tooltip("Section Operation")
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
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(section("Pinned").child(board_renderer::render_item_list(
                                &pinned_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                            )))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            this.child(board_renderer::render_no_section_block(
                                &no_section_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                false,
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
                                SectionBlockOptions { show_inline_edit_delete: true },
                            ))
                        })),
                ),
            )
    }
}
