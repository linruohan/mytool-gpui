//! Todo 设置与快捷键帮助对话框

use gpui::{BorrowAppContext, Context, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{ActiveTheme, WindowExt, switch::Switch, v_flex};

use crate::{
    core::shortcuts::{ShortcutCategory, get_shortcuts_by_category},
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
    )
}
