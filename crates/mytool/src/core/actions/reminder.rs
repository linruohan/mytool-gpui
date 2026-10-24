use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration as StdDuration,
};

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use gpui::App;
use rust_i18n::t;
use todos::{Store, entity::ReminderModel, error::TodoError};

use crate::todo_state::{
    DBState, ErrorNotifier, ReminderChange, ReminderChanges, ReminderNotice, ReminderNotifier,
    TodoPrefs, TodoStore,
};

const MIN_POLL: StdDuration = StdDuration::from_secs(1);
const MAX_POLL: StdDuration = StdDuration::from_secs(30 * 60);

fn reminder_wake() -> &'static tokio::sync::Notify {
    static WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();
    &WAKE
}

/// 新增、删除、延后或开关提醒后立刻打断等待。
pub fn poke_reminder_watcher() {
    reminder_wake().notify_one();
}

async fn wait_for_next_scan(cx: &gpui::AsyncApp, delay: StdDuration) {
    let sleep = cx.background_executor().timer(delay);
    let wake = reminder_wake().notified();
    tokio::pin!(sleep);
    tokio::pin!(wake);
    tokio::select! {
        _ = &mut sleep => {},
        _ = &mut wake => {},
    }
}

fn notify_error(cx: &mut gpui::AsyncApp, message: String) {
    let _ = cx.update_global::<ErrorNotifier, _>(|notifier, _| {
        notifier.set_error(message);
    });
}

pub fn add_reminder(reminder: ReminderModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.insert_reminder(reminder).await })
            .await
        {
            Ok(Ok(_)) => poke_reminder_watcher(),
            Ok(Err(e)) => {
                tracing::error!("add_reminder failed: {:?}", e);
                notify_error(cx, t!("todo.error.add_reminder", error => e.to_string()).to_string());
            },
            Err(join_err) => tracing::error!("add_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn delete_reminder(reminder_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.delete_reminder(&reminder_id).await })
            .await
        {
            Ok(Ok(_)) => poke_reminder_watcher(),
            Ok(Err(e)) => {
                tracing::error!("delete_reminder failed: {:?}", e);
                notify_error(
                    cx,
                    t!("todo.error.delete_reminder", error => e.to_string()).to_string(),
                );
            },
            Err(join_err) => tracing::error!("delete_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn parse_reminder_due(due: &str) -> Option<NaiveDateTime> {
    let due = due.trim();
    NaiveDateTime::parse_from_str(due, "%Y-%m-%d %H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(due, "%Y-%m-%dT%H:%M:%S").ok())
        .or_else(|| {
            NaiveDate::parse_from_str(due, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(9, 0, 0))
        })
}

/// 距下一次到期的等待。新增、删除、延后和开关会通过 [`poke_reminder_watcher`] 提前唤醒。
fn poll_delay(now: NaiveDateTime, next_due: Option<NaiveDateTime>) -> StdDuration {
    let Some(next_due) = next_due else {
        return MAX_POLL;
    };
    let Ok(delta) = (next_due - now).to_std() else {
        return MIN_POLL;
    };
    delta.clamp(MIN_POLL, MAX_POLL)
}

struct DueReminder {
    reminder_id: String,
    item_id: String,
}

struct ReminderScan {
    now: NaiveDateTime,
    next_due: Option<NaiveDateTime>,
    due: Vec<DueReminder>,
}

fn partition_reminders(
    reminders: Vec<ReminderModel>,
    now: NaiveDateTime,
) -> (Vec<ReminderModel>, Option<NaiveDateTime>) {
    let mut due = Vec::new();
    let mut next_due = None;
    for reminder in reminders {
        let Some(dt) = reminder.due.as_deref().and_then(parse_reminder_due) else {
            continue;
        };
        if dt <= now {
            due.push(reminder);
        } else {
            next_due = Some(next_due.map_or(dt, |prev: NaiveDateTime| prev.min(dt)));
        }
    }
    (due, next_due)
}

async fn scan_due_reminders(store: Arc<Store>) -> Result<ReminderScan, TodoError> {
    let now = Local::now().naive_local();
    let reminders = store.get_schedule_reminders(now).await?;
    let (due_models, next_due) = partition_reminders(reminders, now);
    let due = due_models
        .into_iter()
        .map(|reminder| DueReminder {
            reminder_id: reminder.id,
            item_id: reminder.item_id.unwrap_or_default(),
        })
        .collect();
    Ok(ReminderScan { now, next_due, due })
}

async fn load_missing_titles(
    store: Arc<Store>,
    ids: Vec<String>,
) -> Result<HashMap<String, String>, TodoError> {
    store.get_contents_by_ids(&ids).await
}

fn cached_titles(cx: &gpui::AsyncApp, ids: &[String]) -> HashMap<String, String> {
    cx.read_global::<TodoStore, _>(|store, _| {
        ids.iter()
            .filter_map(|id| {
                let content = store.get_item(id)?.content.clone();
                Some((id.clone(), content))
            })
            .collect()
    })
}

fn reminder_title(item_id: &str, titles: &HashMap<String, String>) -> String {
    if item_id.is_empty() {
        return t!("todo.reminder.task_fallback").to_string();
    }
    titles.get(item_id).cloned().unwrap_or_else(|| t!("todo.reminder.task_fallback").to_string())
}

/// 后台轮询到期提醒，写入 ReminderNotifier 供窗口弹出
pub fn start_reminder_watcher(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            let enabled = cx.read_global::<TodoPrefs, _>(|prefs, _| prefs.reminders_enabled);
            if !enabled {
                wait_for_next_scan(&cx, MAX_POLL).await;
                continue;
            }

            let db_state = cx.read_global::<DBState, _>(|s, _| s.clone());
            let scanned = db_state
                .spawn_store_op(|store| async move { scan_due_reminders(store).await })
                .await;

            let delay = match scanned {
                Ok(Ok(scan)) => {
                    let mut seen = HashSet::new();
                    let ids: Vec<String> = scan
                        .due
                        .iter()
                        .filter(|row| !row.item_id.is_empty() && seen.insert(row.item_id.clone()))
                        .map(|row| row.item_id.clone())
                        .collect();
                    let mut titles = cached_titles(&cx, &ids);
                    let missing: Vec<String> =
                        ids.into_iter().filter(|id| !titles.contains_key(id)).collect();
                    if !missing.is_empty() {
                        let loaded = db_state
                            .spawn_store_op(|store| async move {
                                load_missing_titles(store, missing).await
                            })
                            .await;
                        if let Ok(Ok(fetched)) = loaded {
                            titles.extend(fetched);
                        }
                    }
                    for row in scan.due {
                        let notice = ReminderNotice {
                            reminder_id: row.reminder_id,
                            title: reminder_title(&row.item_id, &titles),
                            item_id: row.item_id,
                        };
                        // 先交给界面。软删除放在弹出之后，避免窗口还没订上就丢掉这条提醒。
                        let _ = cx.update_global::<ReminderNotifier, _>(|notifier, _| {
                            notifier.push(notice)
                        });
                    }
                    poll_delay(scan.now, scan.next_due)
                },
                Ok(Err(e)) => {
                    tracing::error!("reminder scan failed: {:?}", e);
                    MAX_POLL
                },
                Err(join_err) => {
                    tracing::error!("reminder scan task panicked: {:?}", join_err);
                    MAX_POLL
                },
            };
            wait_for_next_scan(&cx, delay).await;
        }
    })
    .detach();
}

/// 将已弹出的提醒改到若干分钟之后再次触发
pub fn snooze_reminder(reminder_id: String, item_id: String, minutes: i64, cx: &mut App) {
    let due = (Local::now().naive_local() + Duration::minutes(minutes.max(1)))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let db_state = cx.global::<DBState>().clone();
    let release_id = reminder_id.clone();
    let due_for_ui = due.clone();
    cx.spawn(async move |cx| {
        let result = db_state
            .spawn_store_op(move |store| async move {
                store.reschedule_reminder(&reminder_id, due).await
            })
            .await;
        let _ = cx.update_global::<ReminderNotifier, _>(|notifier, _| {
            notifier.release(&release_id);
        });
        match result {
            Ok(Ok(())) => {
                poke_reminder_watcher();
                let _ = cx.update_global::<ReminderChanges, _>(|changes, _| {
                    changes.push(ReminderChange::Rescheduled {
                        reminder_id: release_id.clone(),
                        item_id,
                        due: due_for_ui,
                    });
                });
            },
            Ok(Err(e)) => {
                tracing::error!("snooze_reminder failed: {:?}", e);
                notify_error(
                    cx,
                    t!("todo.error.snooze_reminder", error => e.to_string()).to_string(),
                );
            },
            Err(join_err) => tracing::error!("snooze_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}

/// 提醒已经显示给用户，软删除以免下一轮再弹出
pub fn acknowledge_reminder(reminder_id: String, item_id: String, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    let release_id = reminder_id.clone();
    cx.spawn(async move |cx| {
        let result = db_state
            .spawn_store_op(
                move |store| async move { store.mark_reminder_notified(&reminder_id).await },
            )
            .await;
        let _ = cx.update_global::<ReminderNotifier, _>(|notifier, _| {
            notifier.release(&release_id);
        });
        match result {
            Ok(Ok(())) => {
                let _ = cx.update_global::<ReminderChanges, _>(|changes, _| {
                    changes.push(ReminderChange::Dismissed { reminder_id: release_id, item_id });
                });
            },
            Ok(Err(e)) => {
                tracing::error!("acknowledge_reminder failed: {:?}", e);
                notify_error(
                    cx,
                    t!("todo.error.acknowledge_reminder", error => e.to_string()).to_string(),
                );
            },
            Err(join_err) => tracing::error!("acknowledge_reminder task panicked: {:?}", join_err),
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_space_separated_due() {
        let dt = parse_reminder_due("2026-09-13 09:00:00").unwrap();
        assert_eq!(dt.format("%H:%M").to_string(), "09:00");
    }

    #[test]
    fn parse_date_only_defaults_morning() {
        let dt = parse_reminder_due("2026-09-13").unwrap();
        assert_eq!(dt.format("%H:%M:%S").to_string(), "09:00:00");
    }

    #[test]
    fn poll_delay_tracks_the_next_due_within_bounds() {
        let now =
            NaiveDateTime::parse_from_str("2026-10-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(poll_delay(now, None), StdDuration::from_secs(30 * 60));
        assert_eq!(poll_delay(now, Some(now + Duration::seconds(10))), StdDuration::from_secs(10));
        assert_eq!(
            poll_delay(now, Some(now + Duration::milliseconds(200))),
            StdDuration::from_secs(1)
        );
        assert_eq!(
            poll_delay(now, Some(now + Duration::hours(2))),
            StdDuration::from_secs(30 * 60)
        );
        assert_eq!(poll_delay(now, Some(now - Duration::seconds(1))), StdDuration::from_secs(1));
    }

    #[test]
    fn partition_keeps_due_and_nearest_future() {
        let now =
            NaiveDateTime::parse_from_str("2026-10-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let mk = |id: &str, due: &str| ReminderModel {
            id: id.into(),
            due: Some(due.into()),
            ..Default::default()
        };
        let (due, next) = partition_reminders(
            vec![
                mk("past", "2026-10-24 09:00:00"),
                mk("soon", "2026-10-24 10:05:00"),
                mk("later", "2026-10-24 12:00:00"),
                mk("blank", ""),
            ],
            now,
        );
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "past");
        assert_eq!(next.unwrap().format("%H:%M").to_string(), "10:05");
    }
}
