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
use rust_i18n::t;
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};

use crate::{
    core::shortcuts::{ShortcutCategory, get_shortcuts_by_category},
    todo_actions::{batch_update_items, update_item_optimistic},
    todo_state::{TodoPrefs, TodoStore},
};

fn startup_boards() -> [(String, u8); 6] {
    [
        (t!("todo.board.inbox").to_string(), 0),
        (t!("todo.board.today").to_string(), 1),
        (t!("todo.board.scheduled").to_string(), 2),
        (t!("todo.board.labels").to_string(), 3),
        (t!("todo.board.pin").to_string(), 4),
        (t!("todo.board.completed").to_string(), 5),
    ]
}

pub fn show_todo_settings_dialog<T: Render>(window: &mut Window, cx: &mut Context<T>) {
    let reminders = cx.global::<TodoPrefs>().reminders_enabled;
    let confirm = cx.global::<TodoPrefs>().confirm_on_delete;
    let sound = cx.global::<TodoPrefs>().complete_sound;
    let startup = cx.global::<TodoPrefs>().startup_board;
    let boards = startup_boards();
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog
            .title(t!("todo.settings.title").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex()
                    .gap_3()
                    .p_2()
                    .w(px(400.))
                    .child(
                        Switch::new("pref-reminders")
                            .label(t!("todo.settings.reminders").to_string())
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
                            .label(t!("todo.settings.confirm_delete").to_string())
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
                            .label(t!("todo.settings.complete_sound").to_string())
                            .checked(sound)
                            .on_click(|checked, _, cx| {
                                cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                    prefs.complete_sound = *checked;
                                    prefs.save();
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("todo.settings.ui_scale").to_string()),
                    )
                    .child({
                        let percent = (TodoPrefs::clamp_ui_scale(cx.global::<TodoPrefs>().ui_scale)
                            * 100.0)
                            .round() as i32;
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("pref-zoom-out")
                                    .small()
                                    .ghost()
                                    .label(t!("todo.settings.zoom_out").to_string())
                                    .on_click(|_, window, cx| {
                                        cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                            prefs.apply_ui_scale_delta(-0.1);
                                        });
                                        let rem = cx.global::<TodoPrefs>().rem_px();
                                        let percent = (TodoPrefs::clamp_ui_scale(
                                            cx.global::<TodoPrefs>().ui_scale,
                                        ) * 100.0)
                                            .round()
                                            as i32;
                                        window.set_rem_size(px(rem));
                                        window.push_notification(
                                            t!(
                                                "todo.settings.ui_scale_set",
                                                percent => percent.to_string()
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    }),
                            )
                            .child(
                                Button::new("pref-zoom-reset")
                                    .small()
                                    .ghost()
                                    .label(format!(
                                        "{} {}%",
                                        t!("todo.settings.zoom_reset"),
                                        percent
                                    ))
                                    .on_click(|_, window, cx| {
                                        cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                            prefs.ui_scale = 1.0;
                                            prefs.save();
                                        });
                                        window.set_rem_size(px(cx.global::<TodoPrefs>().rem_px()));
                                        window.push_notification(
                                            t!(
                                                "todo.settings.ui_scale_set",
                                                percent => "100"
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    }),
                            )
                            .child(
                                Button::new("pref-zoom-in")
                                    .small()
                                    .ghost()
                                    .label(t!("todo.settings.zoom_in").to_string())
                                    .on_click(|_, window, cx| {
                                        cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                            prefs.apply_ui_scale_delta(0.1);
                                        });
                                        let rem = cx.global::<TodoPrefs>().rem_px();
                                        let percent = (TodoPrefs::clamp_ui_scale(
                                            cx.global::<TodoPrefs>().ui_scale,
                                        ) * 100.0)
                                            .round()
                                            as i32;
                                        window.set_rem_size(px(rem));
                                        window.push_notification(
                                            t!(
                                                "todo.settings.ui_scale_set",
                                                percent => percent.to_string()
                                            )
                                            .to_string(),
                                            cx,
                                        );
                                    }),
                            )
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("todo.settings.startup").to_string()),
                    )
                    .child(h_flex().gap_1().flex_wrap().children(boards.iter().map(
                        |(name, ix)| {
                            let selected = startup == *ix;
                            let ix = *ix;
                            let name = name.clone();
                            Button::new(("startup-board", ix as usize))
                                .small()
                                .label(name.clone())
                                .when(selected, |this| this.primary())
                                .when(!selected, |this| this.ghost())
                                .on_click(move |_, window, cx| {
                                    cx.update_global::<TodoPrefs, _>(|prefs, _| {
                                        prefs.startup_board = ix;
                                        prefs.save();
                                    });
                                    window.push_notification(
                                        t!("todo.settings.startup_opened", name => name.as_str())
                                            .to_string(),
                                        cx,
                                    );
                                })
                        },
                    ))),
            )
    });
}

pub fn show_todo_help_dialog<T: Render>(window: &mut Window, cx: &mut Context<T>) {
    let lines: Vec<String> =
        [
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
            rows.extend(get_shortcuts_by_category(cat).into_iter().filter(is_bound_shortcut).map(
                |s| format!("  {}  {}", s.key.replace("cmd", "Ctrl"), s.localized_description()),
            ));
            rows
        })
        .collect();

    window.open_dialog(cx, move |dialog, _, cx| {
        dialog.title(t!("todo.help.title").to_string()).overlay(true).overlay_closable(true).child(
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
            | "RedoLastTask"
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
            | "BatchMoveSelected"
            | "NextView"
            | "PreviousView"
            | "GoBack"
            | "GoForward"
            | "FilterByLabel"
            | "FilterByProject"
            | "FilterByPriority"
            | "ClearFilters"
            | "ShowAllTasks"
            | "NewSection"
            | "EditSection"
            | "DeleteSection"
            | "EditProject"
            | "DeleteProject"
            | "ArchiveProject"
            | "ToggleProjectFavorite"
            | "ToggleLabelFavorite"
            | "ToggleFullscreen"
            | "ZoomIn"
            | "ZoomOut"
            | "ResetZoom"
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
            .title(t!("todo.due.set_title").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                DatePicker::new(&picker)
                    .cleanable(true)
                    .placeholder(t!("todo.due.placeholder").to_string()),
            )
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
                    window.push_notification(t!("todo.due.updated").to_string(), cx);
                    true
                }
            })
    });
}

pub fn show_move_to_project_dialog<T: Render>(
    window: &mut Window,
    cx: &mut Context<T>,
    items: Vec<Arc<ItemModel>>,
) {
    if items.is_empty() {
        return;
    }
    let count = items.len();
    let projects = cx.global::<TodoStore>().projects_for_sidebar();
    window.open_dialog(cx, move |dialog, _, _cx| {
        dialog
            .title(t!("todo.project.move_title").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .w(px(280.))
                    .max_h(px(360.))
                    .child({
                        let items = items.clone();
                        Button::new("move-inbox")
                            .small()
                            .ghost()
                            .label(t!("todo.board.inbox").to_string())
                            .on_click(move |_, window, cx| {
                                apply_move_to_project(&items, None, cx);
                                window.push_notification(
                                    if count == 1 {
                                        t!("todo.project.moved_inbox").to_string()
                                    } else {
                                        t!("todo.project.moved_inbox_n", count => count).to_string()
                                    },
                                    cx,
                                );
                                window.close_dialog(cx);
                            })
                    })
                    .children(projects.iter().enumerate().map(|(ix, (project, nested))| {
                        let items = items.clone();
                        let project = project.clone();
                        let label = if *nested {
                            format!("  {}", project.name)
                        } else {
                            project.name.clone()
                        };
                        Button::new(("move-project", ix)).small().ghost().label(label).on_click(
                            move |_, window, cx| {
                                apply_move_to_project(&items, Some(project.id.clone()), cx);
                                window.push_notification(
                                    if count == 1 {
                                        t!("todo.project.moved", name => project.name.as_str())
                                            .to_string()
                                    } else {
                                        t!("todo.project.moved_n", count => count, name => project.name.as_str())
                                            .to_string()
                                    },
                                    cx,
                                );
                                window.close_dialog(cx);
                            },
                        )
                    })),
            )
    });
}

fn apply_move_to_project(items: &[Arc<ItemModel>], project_id: Option<String>, cx: &mut App) {
    let updated = items
        .iter()
        .map(|item| {
            let mut next = (**item).clone();
            next.project_id = project_id.clone();
            next.section_id = None;
            Arc::new(next)
        })
        .collect();
    batch_update_items(updated, cx);
}

pub fn show_filter_label_dialog<T: Render, F>(window: &mut Window, cx: &mut Context<T>, on_pick: F)
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    let labels = cx.global::<TodoStore>().labels_for_picker();
    if labels.is_empty() {
        window.push_notification(t!("todo.empty.labels_title").to_string(), cx);
        return;
    }
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(t!("todo.filter.labels").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(v_flex().gap_1().p_2().w(px(260.)).max_h(px(360.)).children(
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
            ))
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
        dialog
            .title(t!("todo.filter.projects").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .w(px(280.))
                    .max_h(px(360.))
                    .child({
                        let on_pick = on_pick.clone();
                        Button::new("filter-inbox")
                            .small()
                            .ghost()
                            .label(t!("todo.board.inbox").to_string())
                            .on_click(move |_, window, cx| {
                                on_pick(None, window, cx);
                                window.close_dialog(cx);
                            })
                    })
                    .children(projects.iter().enumerate().map(|(ix, (project, nested))| {
                        let project = project.clone();
                        let on_pick = on_pick.clone();
                        let label = if *nested {
                            format!("  {}", project.name)
                        } else {
                            project.name.clone()
                        };
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
        dialog
            .title(t!("todo.filter.priority").to_string())
            .overlay(true)
            .overlay_closable(true)
            .child(
                v_flex().gap_1().p_2().w(px(220.)).children(
                    [
                        (ItemPriority::HIGH, "p1", t!("todo.priority.high").to_string()),
                        (ItemPriority::MEDIUM, "p2", t!("todo.priority.medium").to_string()),
                        (ItemPriority::LOW, "p3", t!("todo.priority.low").to_string()),
                        (ItemPriority::NONE, "p4", t!("todo.priority.none").to_string()),
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
