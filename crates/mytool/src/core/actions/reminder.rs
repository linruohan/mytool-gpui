use chrono::{NaiveDate, NaiveDateTime};
use gpui::App;
use todos::entity::ReminderModel;

use crate::todo_state::{DBState, ErrorNotifier, ReminderNotifier, TodoPrefs};

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
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                tracing::error!("add_reminder failed: {:?}", e);
                notify_error(cx, format!("添加提醒失败：{e}"));
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
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                tracing::error!("delete_reminder failed: {:?}", e);
                notify_error(cx, format!("删除提醒失败：{e}"));
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

fn reminder_is_due(reminder: &ReminderModel, now: NaiveDateTime) -> bool {
    reminder.due.as_deref().and_then(parse_reminder_due).is_some_and(|dt| dt <= now)
}

/// 后台轮询到期提醒，写入 ReminderNotifier 供窗口弹出
pub fn start_reminder_watcher(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            let enabled = cx.read_global::<TodoPrefs, _>(|prefs, _| prefs.reminders_enabled);
            if !enabled {
                let db_state = cx.read_global::<DBState, _>(|s, _| s.clone());
                let _ = db_state
                    .spawn_store_op(|_| async {
                        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                        Ok(())
                    })
                    .await;
                continue;
            }

            let db_state = cx.read_global::<DBState, _>(|s, _| s.clone());

            let due_list = db_state
                .spawn_store_op(|store| async move {
                    let reminders = store.get_active_reminders().await?;
                    let now = chrono::Local::now().naive_local();
                    Ok(reminders
                        .into_iter()
                        .filter(|r| reminder_is_due(r, now))
                        .collect::<Vec<ReminderModel>>())
                })
                .await;

            if let Ok(Ok(list)) = due_list {
                for reminder in list {
                    let reminder_id = reminder.id.clone();
                    let item_id = reminder.item_id.clone().unwrap_or_default();
                    let title = db_state
                        .spawn_store_op(move |store| async move {
                            Ok(store
                                .get_item(&item_id)
                                .await
                                .map(|item| item.content)
                                .unwrap_or_else(|| "任务".to_string()))
                        })
                        .await
                        .ok()
                        .and_then(Result::ok)
                        .unwrap_or_else(|| "任务".to_string());

                    let _ =
                        db_state
                            .spawn_store_op({
                                let reminder_id = reminder_id.clone();
                                move |store| async move {
                                    store.mark_reminder_notified(&reminder_id).await
                                }
                            })
                            .await;

                    let _ = cx.update_global::<ReminderNotifier, _>(|notifier, _| {
                        notifier.push(format!("提醒：{title}"));
                    });
                }
            }

            let _ = db_state
                .spawn_store_op(|_| async {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    Ok(())
                })
                .await;
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
}
