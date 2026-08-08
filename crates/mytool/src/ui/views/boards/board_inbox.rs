//! InboxBoard - 收件箱视图
//!
//! 显示所有未完成且无项目的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::cell::Cell;

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
    /// 跟踪当前 item_rows 对应的 item id 列表（用于增量更新）
    item_row_ids: Vec<String>,
    /// 🚀 7.0修复：脏标记（当 TodoStore 数据变化时设为 true）
    pending_refresh: Cell<bool>,
    /// 🚀 7.0修复：标记观察者是否已注册（避免初始化阶段循环触发）
    observer_registered: Cell<bool>,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let base = BoardBase::new(window, cx);

        // 🚀 7.0修复：不在 new() 中注册 observe_global！
        // 原因：在初始化阶段注册会导致与异步冷加载产生竞争条件 → 主线程冻结
        // 修复：延迟到首次 render() 时通过 begin_pending_refresh 注册

        Self {
            base,
            item_row_ids: Vec::new(),
            pending_refresh: Cell::new(false),
            observer_registered: Cell::new(false),
        }
    }

    /// 🚀 7.0修复：在 render() 中执行实际的增量更新
    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let has_store_data = if !self.pending_refresh.get() && self.base.item_rows.is_empty() {
            let cache = cx.global::<crate::core::state::QueryCache>();
            let items = cx.global::<TodoStore>().inbox_items_cached(cache);
            let has = !items.is_empty();
            if has {
                tracing::info!(
                    "📭 [InboxBoard] ⚡ 首次渲染兜底: TodoStore 已有 {} 条数据，强制触发刷新！",
                    items.len()
                );
            }
            has
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
            crate::core::state::ChangeMask::affects_inbox,
            |this| &this.pending_refresh,
        ) {
            return;
        }

        let cache = cx.global::<crate::core::state::QueryCache>();
        let state_items = cx.global::<TodoStore>().inbox_items_cached(cache);

        tracing::info!(
            "📭 [InboxBoard] 刷新数据: state_items={}, TodoStore.all_items={}",
            state_items.len(),
            cx.global::<TodoStore>().all_items.len()
        );

        // inbox_items 已过滤未完成，直接使用缓存切片
        self.base.diff_update_item_rows(state_items.as_slice(), &mut self.item_row_ids, window, cx);

        self.base.no_section_items.clear();
        self.base.section_items_map.clear();
        self.base.pinned_items.clear();

        for (i, item) in state_items.iter().enumerate() {
            if item.pinned {
                self.base.pinned_items.push((i, item.clone()));
            } else {
                match item.section_id.as_deref() {
                    None | Some("") => self.base.no_section_items.push((i, item.clone())),
                    Some(sid) => self
                        .base
                        .section_items_map
                        .entry(sid.to_string())
                        .or_default()
                        .push((i, item.clone())),
                }
            }
        }

        tracing::info!(
            "📭 [InboxBoard] 分类结果: pinned={}, no_section={}, sections={}",
            self.base.pinned_items.len(),
            self.base.no_section_items.len(),
            self.base.section_items_map.len()
        );

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
        // 🚀 7.0修复：在 render 开头处理待执行的刷新操作
        tracing::debug!(
            "📭 [InboxBoard] render() 调用: item_rows={}, pinned={}, no_section={}, sections={}",
            self.base.item_rows.len(),
            self.base.pinned_items.len(),
            self.base.no_section_items.len(),
            self.base.section_items_map.len()
        );
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let sections = cx.global::<TodoStore>().sections.clone();
        let pinned_items = self.base.pinned_items.clone();
        let no_section_items = self.base.no_section_items.clone();
        let section_items_map = self.base.section_items_map.clone();
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
