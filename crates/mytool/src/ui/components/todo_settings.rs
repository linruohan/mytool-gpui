//! Todo 设置与快捷键帮助对话框

use std::sync::Arc;

use gpui::{AppContext, BorrowAppContext, Context, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{
    ActiveTheme, WindowExt,
    date_picker::{DatePicker, DatePickerState},
    switch::Switch,
    v_flex,
};
use todos::entity::ItemModel;

use crate::{
    core::shortcuts::{ShortcutCategory, get_shortcuts_by_category},
    todo_actions::update_item_optimistic,
    todo_state::TodoPrefs,
};

pub fn show_todo_settings_dialog<T: Render>(window: &mut Window, cx: &mut Context<T>) {
    let reminders = cx.global::<TodoPrefs>().reminders_enabled;
    let confirm = cx.global::<TodoPrefs>().confirm_on_delete;
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.title("Todo 设置").overlay(true).overlay_closable(true).child(
            v_flex()
                .gap_3()
                .p_2()
                .w(px(360.))
                .child(
                    Switch::new("pref-reminders")
                        .label("到期弹出提醒")
                        .checked(reminders)
                        .on_click(|checked, _, cx| {
                            cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                prefs.reminders_enabled = *checked;
                                prefs.save();
                            });
                        }),
                )
                .child(
                    Switch::new("pref-confirm-delete")
                        .label("删除前确认")
                        .checked(confirm)
                        .on_click(|checked, _, cx| {
                            cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                prefs.confirm_on_delete = *checked;
                                prefs.save();
                            });
                        }),
                ),
        )
    });
}

pub fn show_todo_help_dialog<T: Render>(window: &mut Window, cx: &mut Context<T>) {
    let lines: Vec<String> = [
        ShortcutCategory::Task,
        ShortcutCategory::Navigation,
        ShortcutCategory::Search,
        ShortcutCategory::Selection,
        ShortcutCategory::View,
        ShortcutCategory::Project,
    ]
    .into_iter()
    .flat_map(|cat| {
        let mut rows = vec![cat.name().to_string()];
        rows.extend(
            get_shortcuts_by_category(cat)
                .into_iter()
                .filter(is_bound_shortcut)
                .map(|s| format!("  {}  {}", s.key.replace("cmd", "Ctrl"), s.description)),
        );
        rows
    })
    .collect();

    window.open_dialog(cx, move |dialog, _, cx| {
        dialog.title("快捷键").overlay(true).overlay_closable(true).child(
            v_flex().gap_1().p_2().w(px(420.)).max_h(px(480.)).children(lines.iter().map(|line| {
                div()
                    .text_sm()
                    .text_color(if line.starts_with("  ") {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(line.clone())
            })),
        )
    });
}

fn is_bound_shortcut(s: &crate::core::shortcuts::ShortcutConfig) -> bool {
    matches!(
        s.action,
        "NewTask"
            | "SearchTasks"
            | "ShowInbox"
            | "ShowToday"
            | "ShowScheduled"
            | "ShowLabels"
            | "ShowPinned"
            | "ShowCompleted"
            | "SelectAllTasks"
            | "DeselectAll"
            | "BatchCompleteSelected"
            | "BatchDeleteSelected"
            | "MoveTaskUp"
            | "MoveTaskDown"
            | "OpenSettings"
            | "OpenHelp"
            | "UndoLastTask"
            | "EditTask"
            | "DeleteTask"
            | "ToggleTaskComplete"
            | "DuplicateTask"
            | "ToggleTaskPin"
            | "SetDueDate"
            | "SelectPreviousTask"
            | "SelectNextTask"
            | "ToggleSidebar"
            | "NewProject"
    )
}

pub fn show_set_due_dialog<T: Render>(
    window: &mut Window,
    cx: &mut Context<T>,
    item: Arc<ItemModel>,
) {
    let picker = cx.new(|cx| {
        let mut picker = DatePickerState::new(window, cx);
        if let Some(date) = item.due_date_naive() {
            picker.set_date(date, window, cx);
        }
        picker
    });
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("设置截止日期")
            .overlay(true)
            .overlay_closable(true)
            .child(DatePicker::new(&picker).cleanable(true).placeholder("截止日期"))
            .on_ok({
                let picker = picker.clone();
                let item = item.clone();
                move |_, window, cx| {
                    let ymd = picker.read(cx).date().format("%Y-%m-%d").map(|s| s.to_string());
                    let mut updated = (*item).clone();
                    match ymd {
                        Some(ymd) => {
                            let mut due = item.due_date().unwrap_or_default();
                            let time = due
                                .date
                                .split_once(' ')
                                .map(|(_, t)| t)
                                .filter(|t| !t.is_empty())
                                .unwrap_or("00:00:00");
                            due.date = format!("{ymd} {time}");
                            updated.set_due_date(Some(due));
                        },
                        None => updated.set_due_date(None),
                    }
                    update_item_optimistic(Arc::new(updated), cx);
                    window.push_notification("已更新日期", cx);
                    true
                }
            })
    });
}
