//! TodayBoard - 今日任务视图
//!
//! 显示今天需要完成的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, InteractiveElement, MouseButton,
    ParentElement, Render, Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::InputState,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, ItemRowState, ScheduleButtonEvent, ScheduleButtonState, VisualHierarchy,
    core::actions::batch::batch_update_items,
    section,
    todo_actions::{
        add_item, add_section, delete_item, delete_section, update_item, update_section,
    },
    todo_state::TodoStore,
    ui::views::boards::{BoardView, board_renderer, container_board::Board},
};

/// 显示 section 的 schedule popover
fn show_schedule_popover(
    window: &mut Window,
    cx: &mut App,
    section_id: String,
    _view: Entity<TodayBoard>,
) {
    // 获取该 section 下的所有任务
    let store = cx.global::<TodoStore>();
    let section_items: Vec<Arc<todos::entity::ItemModel>> = store
        .all_items
        .iter()
        .filter(|item| item.section_id.as_deref() == Some(&section_id) && !item.checked)
        .cloned()
        .collect();

    if section_items.is_empty() {
        window.push_notification("No items to schedule in this section", cx);
        return;
    }

    // 显示通知，提示用户选择日期
    window.push_notification(
        format!("Will schedule {} items in section: {}", section_items.len(), section_id),
        cx,
    );

    // TODO: 实现完整的日期选择器 UI
    // 可以考虑使用一个自定义的 popover 组件来显示日期选择器
}

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for TodayBoard {}

pub struct TodayBoard {
    base: BoardBase,
    /// 缓存的 TodoStore 版本号，用于优化性能
    cached_version: usize,
    /// Past Due 分组的 ScheduleButton 状态
    past_due_schedule_button: Entity<ScheduleButtonState>,
    /// ScheduleButton 事件订阅
    _schedule_subscription: Subscription,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);
        base.is_today_board = true;

        // 初始化 Past Due 分组的 ScheduleButton
        let past_due_schedule_button = cx.new(|cx| ScheduleButtonState::new(window, cx));

        // 订阅 ScheduleButton 事件，处理批量重新分配
        let schedule_subscription =
            cx.subscribe_in(&past_due_schedule_button, window, |this, _, event, window, cx| {
                match event {
                    ScheduleButtonEvent::DateSelected(_) => {
                        // 获取选中的日期
                        let due_date = this.past_due_schedule_button.read(cx).due_date.clone();

                        // 获取所有 Past Due 任务
                        let store = cx.global::<TodoStore>();
                        let past_due_items: Vec<Arc<todos::entity::ItemModel>> = store
                            .all_items
                            .iter()
                            .filter(|item| !item.checked && item.is_past_due())
                            .cloned()
                            .collect();

                        if past_due_items.is_empty() {
                            return;
                        }

                        // 为每个任务设置新的日期
                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(Some(due_date.clone()));
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        let count = updated_items.len();

                        // 批量更新
                        batch_update_items(updated_items, cx);

                        window.push_notification(format!("Rescheduled {} items", count), cx);
                    },
                    ScheduleButtonEvent::TimeSelected(_) => {
                        // 时间选择，同样更新所有 Past Due 任务
                        let due_date = this.past_due_schedule_button.read(cx).due_date.clone();

                        let store = cx.global::<TodoStore>();
                        let past_due_items: Vec<Arc<todos::entity::ItemModel>> = store
                            .all_items
                            .iter()
                            .filter(|item| !item.checked && item.is_past_due())
                            .cloned()
                            .collect();

                        if past_due_items.is_empty() {
                            return;
                        }

                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(Some(due_date.clone()));
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        batch_update_items(updated_items, cx);
                    },
                    ScheduleButtonEvent::RecurrencySelected(_) => {
                        // 重复规则选择，同样更新所有 Past Due 任务
                        let due_date = this.past_due_schedule_button.read(cx).due_date.clone();

                        let store = cx.global::<TodoStore>();
                        let past_due_items: Vec<Arc<todos::entity::ItemModel>> = store
                            .all_items
                            .iter()
                            .filter(|item| !item.checked && item.is_past_due())
                            .cloned()
                            .collect();

                        if past_due_items.is_empty() {
                            return;
                        }

                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(Some(due_date.clone()));
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        batch_update_items(updated_items, cx);
                    },
                    ScheduleButtonEvent::Cleared => {
                        // 清除日期，清空所有 Past Due 任务的日期
                        let store = cx.global::<TodoStore>();
                        let past_due_items: Vec<Arc<todos::entity::ItemModel>> = store
                            .all_items
                            .iter()
                            .filter(|item| !item.checked && item.is_past_due())
                            .cloned()
                            .collect();

                        if past_due_items.is_empty() {
                            return;
                        }

                        let mut updated_items = Vec::new();
                        for mut item in past_due_items {
                            let item_mut = Arc::make_mut(&mut item);
                            item_mut.set_due_date(None);
                            updated_items.push(Arc::new(item_mut.clone()));
                        }

                        batch_update_items(updated_items, cx);
                    },
                }
            });

        // 使用 TodoStore 作为数据源（新架构）
        base._subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();

                // 性能优化：检查版本号，只在数据变化时更新
                if this.cached_version == store.version() {
                    return;
                }
                this.cached_version = store.version();

                // 🚀 Today Board 显示：今天任务 + 过期任务 + 无截止日期的任务
                // 使用 all_items 然后在 board_base 中过滤分类
                let state_items = store.all_items
                    .iter()
                    .filter(|item| !item.checked) // 只显示未完成的任务
                    .cloned()
                    .collect::<Vec<_>>();

                this.base.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                this.base.update_items(&state_items);
                cx.notify();
            }),
            cx.observe_global_in::<TodoStore>(window, move |_, _, cx| {
                cx.notify();
            }),
        ];

        Self {
            base,
            cached_version: 0,
            past_due_schedule_button,
            _schedule_subscription: schedule_subscription,
        }
    }

    pub(crate) fn get_selected_item(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<Arc<todos::entity::ItemModel>> {
        // 使用 TodoStore 获取数据
        let item_list = cx.global::<TodoStore>().today_items();
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
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to delete the item?")
                        .on_ok({
                            let item = item.clone();
                            move |_, window: &mut Window, cx| {
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

impl BoardView for TodayBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<gpui::Hsla> {
        vec![gpui::rgb(0x33d17a).into(), gpui::rgb(0x33d17a).into()]
    }

    fn count(cx: &mut gpui::App) -> usize {
        // 使用 TodoStore 获取计数
        cx.global::<TodoStore>().today_items().len()
    }

    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "今天需要完成的任务"
    }

    fn zoomable() -> Option<gpui_component::dock::PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut gpui::App) -> Entity<impl gpui::Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TodayBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for TodayBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        let sections = cx.global::<TodoStore>().sections.clone();
        let pinned_items = self.base.pinned_items.clone();
        let past_due_items = self.base.past_due_items.clone();
        let overdue_items = self.base.overdue_items.clone();
        let no_section_items = self.base.no_section_items.clone();
        let section_items_map = self.base.section_items_map.clone();
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;
        let past_due_schedule_button = self.past_due_schedule_button.clone();

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
                                    .child(<TodayBoard as Board>::icon())
                                    .child(div().text_base().child(<TodayBoard as Board>::title())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<TodayBoard as Board>::description()),
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
                            )
                            .child(
                                Button::new("section-actions")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .label("Add Section")
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
                        // 1. Pinned 分组
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
                                                            PopupMenuItem::new("Show Completed Tasks")
                                                                .on_click(
                                                                    window.listener_for(&view, |_this, _, _window, cx| {
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                        )
                                                    }
                                                }),
                                        ),
                                    )
                                    .child(board_renderer::render_item_section(
                                        "Pinned",
                                        &pinned_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ))
                            )
                        })
                        // 2. Past Due 分组（超过今天但还未完成）
                        .when(!past_due_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("Past Due")
                                    .sub_title(
                                        h_flex().gap_1()
                                            .child(
                                                crate::ui::components::ScheduleButton::new(&past_due_schedule_button),
                                            )
                                            .child(
                                                Button::new("more-past-due")
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EllipsisVertical)
                                                    .dropdown_menu({
                                                        let view = view_clone.clone();
                                                        move |this, window, _cx| {
                                                            this.item(
                                                                PopupMenuItem::new("Show Completed Tasks")
                                                                    .on_click(
                                                                        window.listener_for(&view, |_this, _, _window, cx| {
                                                                            cx.notify();
                                                                        }),
                                                                    ),
                                                            )
                                                        }
                                                    }),
                                            ),
                                    )
                                    .child(board_renderer::render_item_list(
                                        &past_due_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ))
                            )
                        })
                        // 3. Today 分组（今天到期的任务）
                        .when(!overdue_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("Today")
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new("more-today")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::EllipsisVertical)
                                                .dropdown_menu({
                                                    let view = view_clone.clone();
                                                    move |this, window, _cx| {
                                                        this.item(
                                                            PopupMenuItem::new("Show Completed Tasks")
                                                                .on_click(
                                                                    window.listener_for(&view, |_this, _, _window, cx| {
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                        )
                                                    }
                                                }),
                                        ),
                                    )
                                    .child(board_renderer::render_item_section(
                                        "Today",
                                        &overdue_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ))
                            )
                        })
                        // 4. No Section 分组
                        .when(!no_section_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("No Section")
                                    .sub_title(
                                        h_flex().gap_1()
                                            .child(
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
                                            )
                                            .child(
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
                                            ),
                                    )
                                    .child(board_renderer::render_item_list(
                                        &no_section_items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ))
                            )
                        })
                        // 5. Section 分组（带 schedule 按钮）
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
                                        h_flex().gap_1()
                                            // Schedule 按钮
                                            .child(
                                                Button::new(format!(
                                                    "schedule-section-{}",
                                                    section_id
                                                ))
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::Calendar)
                                                .label("Schedule")
                                                .on_click({
                                                    let view = view_clone.clone();
                                                    let section_id = section_id.clone();
                                                    move |_, window, cx| {
                                                        // 打开日期选择器，为该 section 的所有任务设置日期
                                                        show_schedule_popover(
                                                            window,
                                                            cx,
                                                            section_id.clone(),
                                                            view.clone(),
                                                        );
                                                    }
                                                }),
                                            )
                                            // Add Task 按钮
                                            .child(
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
                                            )
                                            // 更多按钮
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
