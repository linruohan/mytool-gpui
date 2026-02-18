//! InboxBoard - 收件箱视图
//!
//! 显示所有未完成且无项目的任务。
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
    input::InputState,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use sea_orm::sqlx::types::uuid;

use crate::{
    Board, BoardBase, ItemRowState, section,
    todo_actions::{
        add_item, add_section, delete_item, delete_section, update_item, update_section,
    },
    todo_state::TodoStore,
    views::boards::{BoardView, board_renderer},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for InboxBoard {}

pub struct InboxBoard {
    base: BoardBase,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);

        // 使用 TodoStore 作为数据源（新架构）
        base._subscriptions = vec![
            // 监听 TodoStore 变化
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                // 从 TodoStore 获取收件箱任务（内存过滤，无需数据库查询）
                let state_items = cx.global::<TodoStore>().inbox_items();

                // 更新 item_rows
                this.base.item_rows = state_items
                    .iter()
                    .filter(|item| !item.checked)
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                // 重新计算 no_section_items 和 section_items_map
                this.base.no_section_items.clear();
                this.base.section_items_map.clear();
                this.base.pinned_items.clear();

                for (i, item) in state_items.iter().enumerate() {
                    if !item.checked {
                        // 先处理置顶任务
                        if item.pinned {
                            this.base.pinned_items.push((i, item.clone()));
                        } else {
                            // 非置顶任务按 section 分类
                            match item.section_id.as_deref() {
                                None | Some("") => {
                                    this.base.no_section_items.push((i, item.clone()))
                                },
                                Some(sid) => {
                                    this.base
                                        .section_items_map
                                        .entry(sid.to_string())
                                        .or_default()
                                        .push((i, item.clone()));
                                },
                            }
                        }
                    }
                }

                // 更新活动索引
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
        ];

        Self { base }
    }

    pub(crate) fn get_selected_item(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<Arc<todos::entity::ItemModel>> {
        // 使用 TodoStore 获取数据
        let item_list = cx.global::<TodoStore>().inbox_items();
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
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }
            self.base.item_info.update(cx, |state, cx| {
                state.set_item(Arc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.base.item_info.clone()
        };

        let config = crate::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        crate::components::show_item_dialog(window, cx, item_info, config, move |item, cx| {
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
                crate::components::show_item_delete_dialog(
                    window,
                    cx,
                    "Are you sure to delete the item?",
                    move |cx| {
                        delete_item(item.clone(), cx);
                    },
                );
            };
        }
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to finish the item?")
                        .on_ok({
                            let item = item.clone();
                            move |_, window, cx| {
                                let mut item_model = (*item).clone();
                                item_model.checked = true;
                                update_item(Arc::new(item_model), cx);
                                window.push_notification("You have finished item ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window, cx| {
                            window.push_notification("You have canceled.", cx);
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
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child(if item.pinned { "Unpin this item?" } else { "Pin this item?" })
                        .on_ok({
                            let item = item.clone();
                            move |_, window, cx| {
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
                        .on_cancel(|_, window, cx| {
                            window.push_notification("Operation canceled.", cx);
                            true
                        })
                });
            };
        }
    }

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        let ori_section = if is_edit {
            sections
                .iter()
                .find(|s| s.id == section_id.clone().unwrap_or_default())
                .map(|s| s.as_ref().clone())
                .unwrap_or_default()
        } else {
            todos::entity::SectionModel::default()
        };

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Section Name"));
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            })
        };

        let config = crate::components::SectionDialogConfig::new(
            if is_edit { "Edit Section" } else { "New Section" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        )
        .with_overlay(false);

        let view = cx.entity().clone();
        crate::components::show_section_dialog(window, cx, name_input, config, move |name, cx| {
            view.update(cx, |_view, cx| {
                let section = Arc::new(todos::entity::SectionModel { name, ..ori_section.clone() });
                if is_edit {
                    update_section(section, cx);
                } else {
                    add_section(section, cx);
                }
                cx.notify();
            });
        });
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        let section_some = sections.iter().find(|s| s.id == section_id).cloned();
        if let Some(section) = section_some {
            let view = cx.entity().clone();
            crate::components::show_section_delete_dialog(
                window,
                cx,
                "Are you sure to delete the section?",
                move |cx| {
                    view.update(cx, |_view, cx| {
                        delete_section(section.clone(), cx);
                        cx.notify();
                    });
                },
            );
        };
    }

    pub fn duplicate_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = format!("{} (copy)", new_section.name);
            add_section(Arc::new(new_section), cx);
            window.push_notification("Section duplicated successfully.", cx);
        }
    }

    pub fn archive_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(Arc::new(updated_section), cx);
            window.push_notification("Section archived successfully.", cx);
        }
    }
}

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
        // 使用 TodoStore 获取计数
        cx.global::<TodoStore>().inbox_items().len()
    }

    fn title() -> &'static str {
        "Inbox"
    }

    fn description() -> &'static str {
        "所有未完成任务"
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
            .gap_4()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(<InboxBoard as Board>::icon())
                                    .child(div().text_base().child(<InboxBoard as Board>::title())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<InboxBoard as Board>::description()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .px_2()
                            .gap_2()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("finish-label")
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
                        .gap_4()
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
                                    .sub_title(h_flex().gap_1().child(
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
                                                        this.show_item_dialog(window, cx, false, None);
                                                        cx.notify();
                                                    })
                                                }
                                            }),
                                    ))
                                    .sub_title(h_flex().gap_1().child(
                                        Button::new("more-no-section")
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::EllipsisVertical)
                                            .dropdown_menu({
                                                let view = view_clone.clone();
                                                move |this, window, _cx| {
                                                    this.item(
                                                        PopupMenuItem::new("+ Add Task").on_click(
                                                            window.listener_for(&view, |this, _, window, cx| {
                                                                this.show_item_dialog(window, cx, false, None);
                                                                cx.notify();
                                                            }),
                                                        ),
                                                    )
                                                    .separator()
                                                    .item(
                                                        PopupMenuItem::new("Show Completed Tasks")
                                                            .on_click(
                                                                window.listener_for(&view, |_this, _, _window, cx| {
                                                                    cx.notify();
                                                                }),
                                                            ),
                                                    )
                                                }
                                            }),
                                    ))
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
                                    .sub_title(h_flex().gap_1().child(
                                        Button::new(format!("add-item-to-section-{}", section_id))
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
                                    ))
                                    .sub_title(
                                        h_flex()
                                            .gap_1()
                                            .child(
                                                Button::new(format!("edit-section-{}", section_id))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EditSymbolic)
                                                    .on_click({
                                                        let view = view_clone.clone();
                                                        let section_id = section_id.clone();
                                                        move |_, window, cx| {
                                                            view.update(cx, |this, cx| {
                                                                this.show_section_dialog(
                                                                    window,
                                                                    cx,
                                                                    Some(section_id.clone()),
                                                                    true,
                                                                );
                                                                cx.notify();
                                                            })
                                                        }
                                                    }),
                                            )
                                            .child(
                                                Button::new(format!("delete-section-{}", section_id))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::UserTrashSymbolic)
                                                    .on_click({
                                                        let view = view_clone.clone();
                                                        let section_id = section_id.clone();
                                                        move |_, window, cx| {
                                                            view.update(cx, |this, cx| {
                                                                this.show_section_delete_dialog(
                                                                    window,
                                                                    cx,
                                                                    section_id.clone(),
                                                                );
                                                                cx.notify();
                                                            })
                                                        }
                                                    }),
                                            )
                                            .child(
                                                Button::new(format!("more-section-{}", section_id))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EllipsisVertical)
                                                    .dropdown_menu({
                                                        let view = view_clone.clone();
                                                        let section_id = section_id.clone();
                                                        move |this, window, _cx| {
                                                            let view = view.clone();
                                                            let section_id1 = section_id.clone();
                                                            let section_id2 = section_id.clone();
                                                            let section_id3 = section_id.clone();
                                                            let section_id4 = section_id.clone();
                                                            let section_id5 = section_id.clone();
                                                            this.item(
                                                                PopupMenuItem::new("+ Add Task").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.show_item_dialog(
                                                                            window,
                                                                            cx,
                                                                            false,
                                                                            Some(section_id1.clone()),
                                                                        );
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
                                                            .separator()
                                                            .item(
                                                                PopupMenuItem::new("Edit Section").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.show_section_dialog(
                                                                            window,
                                                                            cx,
                                                                            Some(section_id2.clone()),
                                                                            true,
                                                                        );
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
                                                            .separator()
                                                            .item(
                                                                PopupMenuItem::new("Duplicate").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.duplicate_section(
                                                                            window,
                                                                            cx,
                                                                            section_id3.clone(),
                                                                        );
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
                                                            .separator()
                                                            .item(
                                                                PopupMenuItem::new("Archive").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.archive_section(
                                                                            window,
                                                                            cx,
                                                                            section_id4.clone(),
                                                                        );
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
                                                            .separator()
                                                            .item(
                                                                PopupMenuItem::new("Delete Section").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.show_section_delete_dialog(
                                                                            window,
                                                                            cx,
                                                                            section_id5.clone(),
                                                                        );
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
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
