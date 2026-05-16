//! InboxBoard - 收件箱视图
//!
//! 显示所有未完成且无项目的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{cell::Cell, sync::Arc};

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
    BoardBase, VisualHierarchy, section,
    todo_actions::{add_section, delete_item, delete_section, update_item, update_section},
    todo_state::TodoStore,
    ui::views::boards::{BoardView, board_renderer, container_board::Board},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for InboxBoard {}

pub struct InboxBoard {
    base: BoardBase,
    /// 观察者 ID（用于细粒度更新）
    #[allow(dead_code)]
    observer_id: Option<u64>,
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
        let mut base = BoardBase::new(window, cx);

        // 注册观察者（细粒度更新）
        let observer_id = {
            let registry = cx.global_mut::<crate::core::state::ObserverRegistry>();
            Some(registry.register(crate::core::state::ViewType::Inbox))
        };

        // 🚀 7.0修复：不在 new() 中注册 observe_global！
        // 原因：在初始化阶段注册会导致与异步冷加载产生竞争条件 → 主线程冻结
        // 修复：延迟到首次 render() 时通过 lazy_init_observer() 注册
        base._subscriptions = vec![];

        Self {
            base,
            observer_id,
            item_row_ids: Vec::new(),
            pending_refresh: Cell::new(false),
            observer_registered: Cell::new(false),
        }
    }

    /// 🚀 7.0修复：延迟注册 TodoStore 观察者（在首次 render 时调用）
    fn lazy_init_observer(&self, cx: &mut Context<Self>) {
        if self.observer_registered.get() {
            return;
        }
        self.observer_registered.set(true);

        tracing::info!("📭 [InboxBoard] 注册 TodoStore 观察者 (延迟注册策略)");

        let _subscription = cx.observe_global::<TodoStore>(move |this, cx| {
            // 只使用只读访问，避免修改全局状态导致循环触发
            let _store = cx.global::<TodoStore>();

            tracing::debug!("📭 [InboxBoard] 观察者回调触发: 设置 pending_refresh=true");

            // 设置脏标记，让 render() 处理实际更新
            this.pending_refresh.set(true);
            cx.notify();
        });
    }

    /// 🚀 7.0修复：在 render() 中执行实际的增量更新
    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // ✅ 修复1：确保观察者在首次 render 时就注册（打破鸡生蛋死锁）
        self.lazy_init_observer(cx);

        // ✅ 修复2：首次渲染兜底 - 解决"观察者注册晚于数据写入"的时序竞态问题
        // 场景：TodoStore 在 InboxBoard 创建前就完成了数据加载，导致观察者错过了第一次写入事件
        if !self.pending_refresh.get() && self.base.item_rows.is_empty() {
            let cache = cx.global::<crate::core::state::QueryCache>();
            let state_items = cx.global::<TodoStore>().inbox_items_cached(cache);

            if !state_items.is_empty() {
                tracing::info!(
                    "📭 [InboxBoard] ⚡ 首次渲染兜底: TodoStore 已有 {} 条数据，强制触发刷新！",
                    state_items.len()
                );
                self.pending_refresh.set(true);
            } else {
                tracing::debug!(
                    "📭 [InboxBoard] 首次渲染兜底: TodoStore 也为空 \
                     (all_items={})，等待数据加载...",
                    cx.global::<TodoStore>().all_items.len()
                );
            }
        }

        if !self.pending_refresh.get() {
            tracing::debug!(
                "📭 [InboxBoard] apply_pending_refresh: pending_refresh=false, 跳过刷新"
            );
            return;
        }
        self.pending_refresh.set(false);

        // 执行实际的刷新逻辑（从原始 observe_global 回调中复制）
        let cache = cx.global::<crate::core::state::QueryCache>();
        let state_items = cx.global::<TodoStore>().inbox_items_cached(cache);

        tracing::info!(
            "📭 [InboxBoard] 刷新数据: state_items={}, TodoStore.all_items={}",
            state_items.len(),
            cx.global::<TodoStore>().all_items.len()
        );

        let filtered_items: Vec<_> =
            state_items.iter().filter(|item| !item.checked).cloned().collect();

        tracing::info!(
            "📭 [InboxBoard] 过滤后: filtered_items={} (未完成任务)",
            filtered_items.len()
        );

        self.base.diff_update_item_rows(&filtered_items, &mut self.item_row_ids, window, cx);

        // 重新计算分类数据
        self.base.no_section_items.clear();
        self.base.section_items_map.clear();
        self.base.pinned_items.clear();

        for (i, item) in filtered_items.iter().enumerate() {
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

        // 更新活动索引
        if let Some(ix) = self.base.active_index {
            if ix >= self.base.item_rows.len() {
                self.base.active_index =
                    if self.base.item_rows.is_empty() { None } else { Some(0) };
            }
        } else if !self.base.item_rows.is_empty() {
            self.base.active_index = Some(0);
        }
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

        let config = crate::ui::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        crate::ui::components::show_item_dialog(window, cx, item_info, config, |_item, _cx| {
            // 🚀 保存已由 save_all_changes 处理，这里不需要再调用 add_item/update_item
        });
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                crate::ui::components::show_item_delete_dialog(
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
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to finish the item?")
                        .on_ok({
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
                                let mut item_model = (*item).clone();
                                item_model.checked = true;
                                update_item(Arc::new(item_model), cx);
                                window.push_notification("You have finished item ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window: &mut Window, cx| {
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
                        .overlay(true)
                        .overlay_closable(true)
                        .child(if item.pinned { "Unpin this item?" } else { "Pin this item?" })
                        .on_ok({
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
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

        let config = crate::ui::components::SectionDialogConfig::new(
            if is_edit { "Edit Section" } else { "New Section" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        )
        .with_overlay(false);

        let view = cx.entity().clone();
        crate::ui::components::show_section_dialog(
            window,
            cx,
            name_input,
            config,
            move |name, cx| {
                view.update(cx, |_view, cx| {
                    let section =
                        Arc::new(todos::entity::SectionModel { name, ..ori_section.clone() });
                    if is_edit {
                        update_section(section, cx);
                    } else {
                        add_section(section, cx);
                    }
                    cx.notify();
                });
            },
        );
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
            crate::ui::components::show_section_delete_dialog(
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
                            .gap(VisualHierarchy::spacing(2.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                                                PopupMenuItem::new("Add Item").icon(IconName::PlusLargeSymbolic).on_click(
                                                    window.listener_for(&view, |this, _, window, cx| {
                                                        this.show_item_dialog(window, cx, false, None);
                                                        cx.notify();
                                                    }),
                                                ),
                                            )
                                            .separator()
                                            .item(
                                                PopupMenuItem::new("Edit Item").icon(IconName::EditSymbolic).on_click(
                                                    window.listener_for(&view, |this, _, window, cx| {
                                                        this.show_item_dialog(window, cx, true, None);
                                                        cx.notify();
                                                    }),
                                                ),
                                            )
                                            .separator()
                                            .item(
                                                PopupMenuItem::new("Delete Item").icon(IconName::UserTrashSymbolic).on_click(
                                                    window.listener_for(&view, |this, _, window, cx| {
                                                        this.show_item_delete_dialog(window, cx);
                                                        cx.notify();
                                                    }),
                                                ),
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
                    ),
            )
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .when(!pinned_items.is_empty(), |this| {
                            this.child(
                                section("Pinned")
                                    .child(board_renderer::render_item_list(
                                        &pinned_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view.clone(),
                                    ))
                            )
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
