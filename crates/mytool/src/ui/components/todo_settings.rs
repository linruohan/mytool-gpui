//! Todo 设置与快捷键帮助对话框

use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, ParentElement, Render, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    switch::Switch,
    v_flex,
};
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};

use crate::{
    core::shortcuts::{ShortcutCategory, get_shortcuts_by_category},
    todo_actions::update_item_optimistic,
    todo_state::{TodoPrefs, TodoStore},
};

const STARTUP_BOARDS: [(&str, u8); 6] =
    [("收件箱", 0), ("今日", 1), ("计划", 2), ("标签", 3), ("置顶", 4), ("已完成", 5)];

pub fn show_todo_settings_dialog<T: Render>(window: &mut Window, cx: &mut Context<T>) {
    let reminders = cx.global::<TodoPrefs>().reminders_enabled;
    let confirm = cx.global::<TodoPrefs>().confirm_on_delete;
    let sound = cx.global::<TodoPrefs>().complete_sound;
    let startup = cx.global::<TodoPrefs>().startup_board;
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog.title("Todo 设置").overlay(true).overlay_closable(true).child(
            v_flex()
                .gap_3()
                .p_2()
                .w(px(400.))
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
                )
                .child(
                    Switch::new("pref-complete-sound")
                        .label("完成任务时播放提示音")
                        .checked(sound)
                        .on_click(|checked, _, cx| {
                            cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                prefs.complete_sound = *checked;
                                prefs.save();
                            });
                        }),
                )
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("启动时打开"))
                .child(h_flex().gap_1().flex_wrap().children(STARTUP_BOARDS.iter().map(
                    |(name, ix)| {
                        let selected = startup == *ix;
                        let ix = *ix;
                        Button::new(("startup-board", ix as usize))
                            .small()
                            .label(*name)
                            .when(selected, |this| this.primary())
                            .when(!selected, |this| this.ghost())
                            .on_click(move |_, window, cx| {
                                cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                    prefs.startup_board = ix;
                                    prefs.save();
                                });
                                window.push_notification(format!("下次启动打开：{name}"), cx);
                            })
                    },
                ))),
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
            | "AddLabel"
            | "SetTaskPriority"
            | "MoveTaskToProject"
            | "NextView"
            | "PreviousView"
            | "FilterByLabel"
            | "FilterByProject"
            | "FilterByPriority"
            | "ClearFilters"
            | "RefreshView"
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

pub fn show_move_to_project_dialog<T: Render>(
    window: &mut Window,
    cx: &mut Context<T>,
    item: Arc<ItemModel>,
) {
    let projects = cx.global::<TodoStore>().projects_for_sidebar();
    window.open_dialog(cx, move |dialog, _, _cx| {
        dialog.title("移动到项目").overlay(true).overlay_closable(true).child(
            v_flex()
                .gap_1()
                .p_2()
                .w(px(280.))
                .max_h(px(360.))
                .child({
                    let item = item.clone();
                    Button::new("move-inbox").small().ghost().label("收件箱").on_click(
                        move |_, window, cx| {
                            let mut updated = (*item).clone();
                            updated.project_id = None;
                            updated.section_id = None;
                            update_item_optimistic(Arc::new(updated), cx);
                            window.push_notification("已移到收件箱", cx);
                            window.close_dialog(cx);
                        },
                    )
                })
                .children(projects.iter().enumerate().map(|(ix, (project, nested))| {
                    let item = item.clone();
                    let project = project.clone();
                    let label =
                        if *nested { format!("  {}", project.name) } else { project.name.clone() };
                    Button::new(("move-project", ix)).small().ghost().label(label).on_click(
                        move |_, window, cx| {
                            let mut updated = (*item).clone();
                            updated.project_id = Some(project.id.clone());
                            updated.section_id = None;
                            update_item_optimistic(Arc::new(updated), cx);
                            window.push_notification(format!("已移到 {}", project.name), cx);
                            window.close_dialog(cx);
                        },
                    )
                })),
        )
    });
}

pub fn show_filter_label_dialog<T: Render, F>(window: &mut Window, cx: &mut Context<T>, on_pick: F)
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    let labels = cx.global::<TodoStore>().labels.clone();
    if labels.is_empty() {
        window.push_notification("还没有标签", cx);
        return;
    }
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.title("按标签过滤").overlay(true).overlay_closable(true).child(
            v_flex().gap_1().p_2().w(px(260.)).max_h(px(360.)).children(
                labels.iter().enumerate().map(|(ix, label)| {
                    let name = label.name.clone();
                    let on_pick = on_pick.clone();
                    Button::new(("filter-label", ix)).small().ghost().label(name.clone()).on_click(
                        move |_, window, cx| {
                            on_pick(name.clone(), window, cx);
                            window.close_dialog(cx);
                        },
                    )
                }),
            ),
        )
    });
}

pub fn show_filter_project_dialog<T: Render, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    on_pick: F,
) where
    F: Fn(Option<Arc<todos::entity::ProjectModel>>, &mut Window, &mut App) + Clone + 'static,
{
    let projects = cx.global::<TodoStore>().projects_for_sidebar();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.title("按项目过滤").overlay(true).overlay_closable(true).child(
            v_flex()
                .gap_1()
                .p_2()
                .w(px(280.))
                .max_h(px(360.))
                .child({
                    let on_pick = on_pick.clone();
                    Button::new("filter-inbox").small().ghost().label("收件箱").on_click(
                        move |_, window, cx| {
                            on_pick(None, window, cx);
                            window.close_dialog(cx);
                        },
                    )
                })
                .children(projects.iter().enumerate().map(|(ix, (project, nested))| {
                    let project = project.clone();
                    let on_pick = on_pick.clone();
                    let label =
                        if *nested { format!("  {}", project.name) } else { project.name.clone() };
                    Button::new(("filter-project", ix)).small().ghost().label(label).on_click(
                        move |_, window, cx| {
                            on_pick(Some(project.clone()), window, cx);
                            window.close_dialog(cx);
                        },
                    )
                })),
        )
    });
}

pub fn show_filter_priority_dialog<T: Render, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    on_pick: F,
) where
    F: Fn(&'static str, &mut Window, &mut App) + Clone + 'static,
{
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.title("按优先级过滤").overlay(true).overlay_closable(true).child(
            v_flex().gap_1().p_2().w(px(220.)).children(
                [
                    (ItemPriority::HIGH, "p1", "高优先级"),
                    (ItemPriority::MEDIUM, "p2", "中优先级"),
                    (ItemPriority::LOW, "p3", "低优先级"),
                    (ItemPriority::NONE, "p4", "无优先级"),
                ]
                .into_iter()
                .enumerate()
                .map(|(ix, (_p, token, label))| {
                    let on_pick = on_pick.clone();
                    Button::new(("filter-priority", ix)).small().ghost().label(label).on_click(
                        move |_, window, cx| {
                            on_pick(token, window, cx);
                            window.close_dialog(cx);
                        },
                    )
                }),
            ),
        )
    });
}
