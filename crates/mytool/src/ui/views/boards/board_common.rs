//! Board 视图共享组件与事件

use std::sync::Arc;

use gpui::{
    App, AppContext, ClickEvent, Context, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, SharedString, Styled, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IndexPath, Sizable, StyledExt, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use todos::entity::ItemModel;

use super::board_base::BoardView;
use crate::{
    ScheduleButtonState, VisualHierarchy,
    core::{
        actions::batch::{
            batch_complete_selected, batch_delete_selected, batch_pin_selected, batch_update_items,
        },
        state::ItemSelection,
    },
    todo_actions::{complete_item_optimistic, delete_item_optimistic, set_item_pinned_optimistic},
    todo_state::TodoStore,
};

/// 所有 Board 共享的任务点击事件
#[derive(Debug, Clone)]
pub enum BoardItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

/// 完成任务的确认文案变体（各 Board 行为不同）
#[derive(Debug, Clone, Copy)]
pub enum FinishItemDialogStyle {
    /// Inbox：「Are you sure to finish the item?」
    Inbox,
    /// Pin / Scheduled：「Mark this item as completed?」
    Standard,
}

impl FinishItemDialogStyle {
    fn message(self) -> String {
        match self {
            Self::Inbox => t!("todo.item.finish_confirm_inbox").to_string(),
            Self::Standard => t!("todo.item.finish_confirm").to_string(),
        }
    }

    fn success_notification(self) -> String {
        match self {
            Self::Inbox => t!("todo.item.finished").to_string(),
            Self::Standard => t!("todo.item.marked_complete").to_string(),
        }
    }

    fn cancel_notification(self) -> String {
        match self {
            Self::Inbox => t!("todo.item.cancelled").to_string(),
            Self::Standard => t!("todo.item.op_cancelled").to_string(),
        }
    }
}

/// 无日期任务在日程分组里的内部键（展示文案走 i18n）。
pub const UNDATED_DATE_KEY: &str = "__undated__";

pub fn weekday_short(n: u32) -> String {
    match n % 7 {
        0 => t!("todo.weekday.sun").to_string(),
        1 => t!("todo.weekday.mon").to_string(),
        2 => t!("todo.weekday.tue").to_string(),
        3 => t!("todo.weekday.wed").to_string(),
        4 => t!("todo.weekday.thu").to_string(),
        5 => t!("todo.weekday.fri").to_string(),
        _ => t!("todo.weekday.sat").to_string(),
    }
}

/// 通用确认对话框
pub fn show_confirm_dialog<T, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    message: impl Into<gpui::SharedString>,
    _confirm_label: &str,
    on_confirm: F,
    success_notification: &str,
    cancel_notification: &str,
) where
    T: Render + 'static,
    F: Fn(&mut App) + Clone + 'static,
{
    let message = message.into();
    let success_notification = success_notification.to_string();
    let cancel_notification = cancel_notification.to_string();
    let on_confirm = on_confirm.clone();

    window.open_dialog(cx, move |dialog, _, _| {
        let on_confirm = on_confirm.clone();
        let message = message.clone();
        let success_notification = success_notification.clone();
        let cancel_notification = cancel_notification.clone();
        dialog
            .overlay(true)
            .overlay_closable(true)
            .child(Alert::warning("confirm-alert", message.clone()).title(t!("todo.confirm")))
            .on_ok({
                let on_confirm = on_confirm.clone();
                let success_notification = success_notification.clone();
                move |_, window: &mut Window, cx| {
                    on_confirm(cx);
                    window.push_notification(success_notification.clone(), cx);
                    true
                }
            })
            .on_cancel({
                let cancel_notification = cancel_notification.clone();
                move |_, window: &mut Window, cx| {
                    window.push_notification(cancel_notification.clone(), cx);
                    true
                }
            })
    });
}

/// 删除任务确认对话框
pub fn show_item_delete_dialog<T>(window: &mut Window, cx: &mut Context<T>, item: Arc<ItemModel>)
where
    T: Render + 'static,
{
    if !cx.global::<crate::core::state::TodoPrefs>().confirm_on_delete {
        delete_item_optimistic(item, cx);
        window.push_notification(t!("todo.item.deleted").to_string(), cx);
        return;
    }
    show_confirm_dialog(
        window,
        cx,
        t!("todo.item.delete_confirm").to_string(),
        &t!("todo.confirm"),
        move |cx| {
            delete_item_optimistic(item.clone(), cx);
        },
        &t!("todo.item.deleted"),
        &t!("todo.item.delete_cancel"),
    );
}

/// 完成任务确认对话框
pub fn show_finish_item_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    item: Arc<ItemModel>,
    style: FinishItemDialogStyle,
) where
    T: Render + 'static,
{
    let success = if item.due_date().and_then(|d| d.next_due_after_completion()).is_some() {
        t!("todo.item.next_occurrence").to_string()
    } else {
        style.success_notification()
    };
    show_confirm_dialog(
        window,
        cx,
        style.message(),
        &t!("todo.confirm"),
        move |cx| {
            complete_item_optimistic(item.clone(), true, cx);
        },
        &success,
        &style.cancel_notification(),
    );
}

/// 置顶/取消置顶任务确认对话框
pub fn show_pin_item_dialog<T>(window: &mut Window, cx: &mut Context<T>, item: Arc<ItemModel>)
where
    T: Render + 'static,
{
    let message = if item.pinned {
        t!("todo.item.unpin_confirm").to_string()
    } else {
        t!("todo.item.pin_confirm").to_string()
    };
    let success = if item.pinned {
        t!("todo.item.unpinned").to_string()
    } else {
        t!("todo.item.pinned").to_string()
    };

    show_confirm_dialog(
        window,
        cx,
        message,
        &t!("todo.confirm"),
        move |cx| {
            set_item_pinned_optimistic(item.clone(), !item.pinned, cx);
        },
        &success,
        &t!("todo.item.op_cancelled"),
    );
}

/// 标记未完成确认对话框（Completed Board）
pub fn show_item_unfinish_dialog<T>(window: &mut Window, cx: &mut Context<T>, item: Arc<ItemModel>)
where
    T: Render + 'static,
{
    show_confirm_dialog(
        window,
        cx,
        t!("todo.item.unfinish_confirm").to_string(),
        &t!("todo.confirm"),
        move |cx| {
            complete_item_optimistic(item.clone(), false, cx);
        },
        &t!("todo.item.unfinished"),
        &t!("todo.item.cancelled"),
    );
}

/// 若当前有选中项，则对其执行回调
pub fn with_selected_item<V, F>(
    active_index: Option<usize>,
    base: &super::board_base::BoardBase,
    cx: &mut Context<V>,
    f: F,
) where
    V: Render,
    F: FnOnce(Arc<ItemModel>, &mut Context<V>),
{
    if let Some(ix) = active_index {
        if let Some(item) = base.get_selected_item_from_index(IndexPath::new(ix), cx) {
            f(item, cx);
        }
    }
}

/// Section 区块操作（供渲染辅助与 listener_for 使用）
pub trait BoardSectionActions: BoardView + Render {
    fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    );

    fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    );

    fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    );

    fn duplicate_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    );

    fn archive_section(&mut self, window: &mut Window, cx: &mut Context<Self>, section_id: String);
}

/// 为 Board 生成委托给 `BoardBase` 的 section 相关固有方法
#[macro_export]
macro_rules! impl_board_section_forwards {
    ($board:ty) => {
        impl $board {
            pub fn show_item_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                is_edit: bool,
                section_id: Option<String>,
            ) {
                self.base.show_item_dialog(window, cx, is_edit, section_id);
            }

            pub fn show_section_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: Option<String>,
                is_edit: bool,
            ) {
                self.base.show_section_dialog(window, cx, section_id, is_edit);
            }

            pub fn show_section_delete_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                $crate::BoardBase::show_section_delete_dialog(window, cx, section_id);
            }

            pub fn duplicate_section(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                self.base.duplicate_section(window, cx, section_id);
            }

            pub fn archive_section(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                self.base.archive_section(window, cx, section_id);
            }
        }
    };
}

/// 将 Board 已有的同名固有方法委托给 `BoardSectionActions`
#[macro_export]
macro_rules! impl_board_section_actions {
    ($board:ty) => {
        impl $crate::ui::views::boards::board_common::BoardSectionActions for $board {
            fn show_item_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                is_edit: bool,
                section_id: Option<String>,
            ) {
                Self::show_item_dialog(self, window, cx, is_edit, section_id);
            }

            fn show_section_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: Option<String>,
                is_edit: bool,
            ) {
                Self::show_section_dialog(self, window, cx, section_id, is_edit);
            }

            fn show_section_delete_dialog(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                Self::show_section_delete_dialog(self, window, cx, section_id);
            }

            fn duplicate_section(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                Self::duplicate_section(self, window, cx, section_id);
            }

            fn archive_section(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
                section_id: String,
            ) {
                Self::archive_section(self, window, cx, section_id);
            }
        }
    };
}

/// 显示 section 的 schedule popover（批量设置 section 内任务日期）
pub fn show_schedule_popover(window: &mut Window, cx: &mut App, section_id: String) {
    let store = cx.global::<TodoStore>();
    let section_items: Vec<Arc<ItemModel>> = store
        .all_items
        .iter()
        .filter(|item| item.section_id.as_deref() == Some(&section_id) && !item.checked)
        .cloned()
        .collect();

    if section_items.is_empty() {
        window.push_notification(t!("todo.section.no_schedulable").to_string(), cx);
        return;
    }

    let schedule_state = cx.new(|cx| ScheduleButtonState::new(window, cx));

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(t!("todo.section.schedule_title").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex()
                    .gap_2()
                    .child(gpui::div().child(t!("todo.section.schedule_hint").to_string()))
                    .child(crate::ui::components::ScheduleButton::new(&schedule_state)),
            )
            .footer(
                gpui_component::dialog::DialogFooter::new()
                    .child(gpui_component::dialog::DialogClose::new().child(
                        Button::new("cancel").label(t!("todo.cancel").to_string()).outline(),
                    ))
                    .child(
                        gpui_component::dialog::DialogAction::new().child(
                            Button::new("schedule")
                                .label(t!("todo.section.schedule").to_string())
                                .primary(),
                        ),
                    ),
            )
            .on_ok({
                let schedule_state = schedule_state.clone();
                let section_items = section_items.clone();
                move |_, window, cx| {
                    let due_date = schedule_state.read(cx).due_date.clone();

                    let mut updated_items = Vec::new();
                    for item in &section_items {
                        let mut item_clone = (**item).clone();
                        item_clone.set_due_date(Some(due_date.clone()));
                        updated_items.push(Arc::new(item_clone));
                    }

                    let count = updated_items.len();

                    batch_update_items(updated_items, cx);

                    window.push_notification(
                        t!("todo.section.scheduled_n", count => count).to_string(),
                        cx,
                    );
                    true
                }
            })
    });
}

/// 渲染 Board 顶部标题栏（左侧 icon + 大标题 + 弱化副标题，右侧操作）
pub fn render_board_header(
    cx: &App,
    icon: IconName,
    title: impl IntoElement,
    description: impl Into<SharedString>,
    count: usize,
    actions: impl IntoElement,
) -> impl IntoElement {
    let description = description.into();
    h_flex()
        .id("header")
        .justify_between()
        .items_start()
        .px(px(16.))
        .pt(px(12.))
        .pb(px(6.))
        .child(
            h_flex()
                .gap(px(8.))
                .items_start()
                .child(
                    Icon::new(icon)
                        .with_size(px(18.))
                        .text_color(cx.theme().muted_foreground)
                        .mt(px(3.)),
                )
                .child(
                    v_flex()
                        .gap(px(2.))
                        .child(
                            h_flex()
                                .gap(px(8.))
                                .items_baseline()
                                .child(gpui::div().text_lg().font_semibold().child(title))
                                .when(count > 0, |this| {
                                    this.child(
                                        gpui::div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(count.to_string()),
                                    )
                                }),
                        )
                        .when(!description.is_empty(), |this| {
                            this.child(
                                gpui::div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description),
                            )
                        }),
                ),
        )
        .child(
            gpui::div()
                .flex()
                .items_center()
                .justify_end()
                .gap(VisualHierarchy::spacing(2.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(actions),
        )
}

/// 多选工具条：完成 / 置顶 / 删除
pub fn render_batch_bar(cx: &App) -> impl IntoElement {
    let count = cx.global::<ItemSelection>().len();
    h_flex().w_full().px(px(16.)).when(count > 0, move |this| {
        this.py(px(6.))
            .gap(px(8.))
            .items_center()
            .child(
                gpui::div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("todo.batch.selected", count => count).to_string()),
            )
            .child(
                Button::new("batch-complete")
                    .small()
                    .ghost()
                    .icon(IconName::CheckSquare)
                    .tooltip(t!("todo.batch.complete").to_string())
                    .on_click(|_, window, cx| {
                        let n = batch_complete_selected(cx);
                        if n > 0 {
                            window.push_notification(
                                t!("todo.batch.completed_n", count => n).to_string(),
                                cx,
                            );
                        }
                    }),
            )
            .child(
                Button::new("batch-pin")
                    .small()
                    .ghost()
                    .icon(IconName::PinSymbolic)
                    .tooltip(t!("todo.batch.pin").to_string())
                    .on_click(|_, window, cx| {
                        let n = batch_pin_selected(cx);
                        if n > 0 {
                            window.push_notification(
                                t!("todo.batch.pinned_n", count => n).to_string(),
                                cx,
                            );
                        }
                    }),
            )
            .child(
                Button::new("batch-delete")
                    .small()
                    .ghost()
                    .icon(IconName::UserTrashSymbolic)
                    .tooltip(t!("todo.batch.delete").to_string())
                    .on_click(|_, window, cx| {
                        let n = cx.global::<ItemSelection>().len();
                        if n == 0 {
                            return;
                        }
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog
                                .overlay(true)
                                .overlay_closable(true)
                                .child(
                                    Alert::warning(
                                        "batch-delete-alert",
                                        t!("todo.batch.delete_confirm", count => n).to_string(),
                                    )
                                    .title(t!("todo.confirm")),
                                )
                                .on_ok(move |_, window: &mut Window, cx| {
                                    let deleted = batch_delete_selected(cx);
                                    if deleted > 0 {
                                        window.push_notification(
                                            t!("todo.batch.deleted_n", count => deleted)
                                                .to_string(),
                                            cx,
                                        );
                                    }
                                    true
                                })
                                .on_cancel(move |_, window: &mut Window, cx| {
                                    window.push_notification(
                                        t!("todo.item.delete_cancel").to_string(),
                                        cx,
                                    );
                                    true
                                })
                        });
                    }),
            )
    })
}

/// 列表底部留白，避免最后几行被悬浮添加按钮挡住
pub const FAB_BOTTOM_PAD: gpui::Pixels = px(56.);

/// 右下角新建任务按钮（叠在内容之上，不挤占列表高度）
pub fn render_add_task_fab(
    id: impl Into<ElementId>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    gpui::div()
        .absolute()
        .bottom(px(16.))
        .right(px(16.))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            Button::new(id)
                .icon(IconName::PlusLargeSymbolic)
                .primary()
                .large()
                .rounded(px(24.))
                .tooltip(t!("todo.item.new").to_string())
                .on_click(on_click),
        )
}
