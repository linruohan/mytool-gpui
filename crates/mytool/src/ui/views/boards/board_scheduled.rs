//! ScheduledBoard - 计划任务视图
//!
//! 显示计划中任务，在其他时间去执行的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, ItemRowState, VisualHierarchy, section,
    todo_actions::{add_item, delete_item, update_item},
    todo_state::TodoStore,
    ui::views::boards::{BoardView, board_renderer, container_board::Board},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for ScheduledBoard {}

pub struct ScheduledBoard {
    base: BoardBase,
    /// 缓存的 TodoStore 版本号，用于优化性能
    cached_version: usize,
}

impl ScheduledBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);

        // 使用 TodoStore 作为数据源（新架构）
        base._subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();

                // 性能优化：检查版本号，只在数据变化时更新
                if this.cached_version == store.version() {
                    return;
                }
                this.cached_version = store.version();

                // 从 TodoStore 获取计划任务（内存过滤，无需数据库查询）
                let state_items = store.scheduled_items();

                this.base.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                this.base.update_items(&state_items);

                if let Some(ix) = this.base.active_index {
                    if ix >= this.base.item_rows.len() {
                        this.base.active_index =
                            if this.base.item_rows.is_empty() { None } else { Some(0) };
                    }
                } else if !this.base.item_rows.is_empty() {
                    this.base.active_index = Some(0);
                }
                cx.notify();
            }),
            cx.observe_global_in::<TodoStore>(window, move |_, _, cx| {
                cx.notify();
            }),
        ];

        Self { base, cached_version: 0 }
    }

    pub(crate) fn get_selected_item(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<Arc<todos::entity::ItemModel>> {
        // 使用 TodoStore 获取数据
        let item_list = cx.global::<TodoStore>().scheduled_items();
        item_list.get(ix.row).cloned()
    }

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        let item_info = if is_edit {
            if let Some(active_index) = self.base.active_index {
                if let Some(item_row) = self.base.item_rows.get(active_index) {
                    item_row.read(cx).item_info.clone()
                } else {
                    self.base.item_info.clone()
                }
            } else {
                self.base.item_info.clone()
            }
        } else {
            let mut ori_item = todos::entity::ItemModel::default();

            // If adding a new item with a section_id, set it
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }

            self.base.item_info.update(cx, |state, cx| {
                state.set_item(std::sync::Arc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.base.item_info.clone()
        };

        let config = crate::ui::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        crate::ui::components::show_item_dialog(window, cx, item_info, config, move |item, cx| {
            if is_edit {
                update_item(item, cx);
            } else {
                add_item(item, cx);
            }
        });
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to delete the item?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
                                let _view = view.clone();
                                delete_item(item.clone(), cx);
                                window.push_notification("You have delete ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window: &mut Window, cx| {
                            window.push_notification("You have canceled delete.", cx);
                            true
                        })
                });
            };
        }
    }

    pub fn show_pin_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .overlay(true)
                        .overlay_closable(true)
                        .child(if item.pinned { "Unpin this item?" } else { "Pin this item?" })
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
                                let _view = view.clone();
                                let mut item_model = (*item).clone();
                                item_model.pinned = !item.pinned;
                                update_item(Arc::new(item_model), cx);
                                window.push_notification(
                                    if item.pinned { "Item unpinned." } else { "Item pinned." },
                                    cx,
                                );
                                true
                            }
                        })
                        .on_cancel(|_, window: &mut Window, cx| {
                            window.push_notification("Operation canceled.", cx);
                            true
                        })
                });
            };
        }
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Mark this item as completed?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
                                let _view = view.clone();
                                let mut item_model = (*item).clone();
                                item_model.checked = true;
                                update_item(Arc::new(item_model), cx);
                                window.push_notification("Item marked as completed.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window: &mut Window, cx| {
                            window.push_notification("Operation canceled.", cx);
                            true
                        })
                });
            };
        }
    }
}

impl BoardView for ScheduledBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for ScheduledBoard {
    fn icon() -> IconName {
        IconName::MonthSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xdc8add).into(), gpui::rgb(0x9141ac).into()]
    }

    fn count(cx: &mut App) -> usize {
        // 使用 TodoStore 获取计数
        cx.global::<TodoStore>().scheduled_items().len()
    }

    fn title() -> &'static str {
        "Scheduled"
    }

    fn description() -> &'static str {
        "计划中任务，在其他时间去执行的任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScheduledBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for ScheduledBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
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
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .p(VisualHierarchy::spacing(3.0))
                    .child(
                        v_flex()
                            .gap(VisualHierarchy::spacing(1.0))
                            .child(
                                h_flex()
                                    .gap(VisualHierarchy::spacing(2.0))
                                    .items_center()
                                    .child(<ScheduledBoard as Board>::icon())
                                    .child(
                                        div().text_base().child(<ScheduledBoard as Board>::title()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<ScheduledBoard as Board>::description()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(VisualHierarchy::spacing(2.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("add-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_dialog(window, cx, false, None);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("edit-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_dialog(window, cx, true, None);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("finish-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::CheckmarkSmallSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_finish_item_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("pin-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PinSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_pin_item_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("delete-item")
                                    .icon(IconName::UserTrashSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_delete_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    ),
            )
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(board_renderer::render_item_section(
                                "Pinned",
                                &pinned_items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("No Section")
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new("add-item-to-no-section")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::PlusLargeSymbolic)
                                                .label("Add Task")
                                                .on_click({
                                                    let view = view_clone.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.show_item_dialog(
                                                                window, cx, false, None,
                                                            );
                                                            cx.notify();
                                                        })
                                                    }
                                                }),
                                        ),
                                    )
                                    .child(board_renderer::render_item_list(
                                        &no_section_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    )),
                            )
                        })
                        .children(sections.iter().filter_map(|sec| {
                            let items = section_items_map.get(&sec.id)?;
                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();
                            let section_id = sec.id.clone();

                            Some(
                                section(sec.name.clone())
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new(format!(
                                                "add-item-to-section-{}",
                                                section_id
                                            ))
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::PlusLargeSymbolic)
                                            .label("Add Task")
                                            .on_click({
                                                let view = view_clone.clone();
                                                let section_id = section_id.clone();
                                                move |_, window, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.show_item_dialog(
                                                            window,
                                                            cx,
                                                            false,
                                                            Some(section_id.clone()),
                                                        );
                                                        cx.notify();
                                                    })
                                                }
                                            }),
                                        ),
                                    )
                                    .child(board_renderer::render_item_list(
                                        items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    )),
                            )
                        })),
                ),
            )
    }
}
