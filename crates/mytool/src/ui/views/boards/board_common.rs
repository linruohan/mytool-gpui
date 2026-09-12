//! Board 视图共享组件与事件

use std::sync::Arc;

use gpui::{
    App, AppContext, ClickEvent, Context, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, Sizable, StyledExt, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    h_flex,
    v_flex,
};
use gpui_kit::assets::IconName;
use todos::entity::ItemModel;

use super::board_base::BoardView;
use crate::{
    ScheduleButtonState, VisualHierarchy,
    core::actions::batch::batch_update_items,
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
    fn message(self) -> &'static str {
        match self {
            Self::Inbox => "Are you sure to finish the item?",
            Self::Standard => "Mark this item as completed?",
        }
    }

    fn success_notification(self) -> &'static str {
        match self {
            Self::Inbox => "You have finished item ok.",
            Self::Standard => "Item marked as completed.",
        }
    }

    fn cancel_notification(self) -> &'static str {
        match self {
            Self::Inbox => "You have canceled.",
            Self::Standard => "Operation canceled.",
        }
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
            .child(Alert::warning("confirm-alert", message.clone()).title("Confirm"))
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
    show_confirm_dialog(
        window,
        cx,
        "Are you sure to delete the item?",
        "Confirm",
        move |cx| {
            delete_item_optimistic(item.clone(), cx);
        },
        "You have delete ok.",
        "You have canceled delete.",
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
    show_confirm_dialog(
        window,
        cx,
        style.message(),
        "Confirm",
        move |cx| {
            complete_item_optimistic(item.clone(), true, cx);
        },
        style.success_notification(),
        style.cancel_notification(),
    );
}

/// 置顶/取消置顶任务确认对话框
pub fn show_pin_item_dialog<T>(window: &mut Window, cx: &mut Context<T>, item: Arc<ItemModel>)
where
    T: Render + 'static,
{
    let message = if item.pinned { "Unpin this item?" } else { "Pin this item?" };
    let success = if item.pinned { "Item unpinned." } else { "Item pinned." };

    show_confirm_dialog(
        window,
        cx,
        message,
        "Confirm",
        move |cx| {
            set_item_pinned_optimistic(item.clone(), !item.pinned, cx);
        },
        success,
        "Operation canceled.",
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
        "Are you sure to mark this item as unfinished?",
        "Confirm",
        move |cx| {
            complete_item_optimistic(item.clone(), false, cx);
        },
        "Item marked as unfinished.",
        "You have canceled.",
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
        window.push_notification("No items to schedule in this section", cx);
        return;
    }

    let schedule_state = cx.new(|cx| ScheduleButtonState::new(window, cx));

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("Schedule Section Tasks")
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex()
                    .gap_2()
                    .child(gpui::div().child("Select date for all tasks in this section:"))
                    .child(crate::ui::components::ScheduleButton::new(&schedule_state)),
            )
            .footer(
                gpui_component::dialog::DialogFooter::new()
                    .child(
                        gpui_component::dialog::DialogClose::new()
                            .child(Button::new("cancel").label("Cancel").outline()),
                    )
                    .child(
                        gpui_component::dialog::DialogAction::new()
                            .child(Button::new("schedule").label("Schedule").primary()),
                    ),
            )
            .on_ok({
                let schedule_state = schedule_state.clone();
                let section_items = section_items.clone();
                let section_id = section_id.clone();
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
                        format!("Scheduled {} tasks in section '{}'", count, section_id),
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
    icon: impl IntoElement,
    title: impl IntoElement,
    description: impl IntoElement,
    _count: usize,
    actions: impl IntoElement,
) -> impl IntoElement {
    h_flex()
        .id("header")
        .justify_between()
        .items_center()
        .px(px(24.))
        .pt(px(18.))
        .pb(px(8.))
        .child(
            h_flex()
                .gap(px(10.))
                .items_baseline()
                .child(icon)
                .child(gpui::div().text_xl().font_semibold().child(title))
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(description),
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

/// 右下角新建任务按钮
pub fn render_add_task_fab(
    id: impl Into<ElementId>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .justify_end()
        .px_6()
        .pb_6()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            Button::new(id)
                .icon(IconName::PlusLargeSymbolic)
                .primary()
                .large()
                .rounded(px(28.))
                .on_click(on_click),
        )
}
