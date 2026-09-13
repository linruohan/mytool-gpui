//! 乐观更新 - 立即更新 UI，异步保存到数据库
//!
//! 这个模块提供了乐观更新的实现，可以显著提升用户体验：
//! 1. 立即更新 UI（乐观更新）
//! 2. 异步保存到数据库（使用 cx.spawn + spawn_db_operation，不阻塞 UI）
//! 3. 自动重试机制 + Store 就绪等待，确保数据可靠落盘
//! 4. 窗口关闭时会等待 DB 操作完成后再退出

use std::sync::Arc;

use gpui::{App, BorrowAppContext};
use rust_i18n::t;
use todos::entity::ItemModel;
use tracing::{debug, error};

use crate::{
    core::{
        error_handler::{AppError, ErrorHandler, validation},
        state::{ErrorNotifier, TodoStore, UndoEntry, UndoStack},
        utils::retry::{self, RetryConfig},
    },
    todo_state::DBState,
};

/// 乐观添加任务
///
/// 1. 立即更新 UI（使用临时 ID）
/// 2. 异步保存到数据库（使用共享连接池 + 重试机制）
/// 3. 自动等待 Store 就绪 + 重试机制，确保数据可靠落盘
///
/// # 返回值
/// - 返回生成的临时 ID，用于更新原始 item 对象
pub fn add_item_optimistic(item: Arc<ItemModel>, cx: &mut App) -> String {
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item_optimistic");
        error!("{}", context.format_user_message());
        cx.update_global::<ErrorNotifier, _>(|notifier, _| {
            notifier.set_error(context.format_user_message());
        });
        return "".to_string();
    }

    // 1. 生成稳定 ID：UI 与数据库使用同一主键，避免随后按 temp_ ID UPDATE 失败
    let item_id = if item.id.is_empty() || item.id.starts_with("temp_") {
        uuid::Uuid::new_v4().to_string()
    } else {
        item.id.clone()
    };
    let mut optimistic_item = (*item).clone();
    optimistic_item.id = item_id.clone();

    debug!("Optimistically adding item with ID: {}, content: '{}'", item_id, item.content);

    cx.update_global::<TodoStore, _>(|store, _| {
        store.add_item(Arc::new(optimistic_item.clone()));
    });

    let db_state = cx.global::<DBState>().clone();
    let db_state_for_labels = db_state.clone();
    let item_for_save = Arc::new(optimistic_item);
    cx.update_global::<UndoStack, _>(|stack, _| {
        stack.record(UndoEntry::Created(item_for_save.clone()));
    });
    let item_id_for_error = item_id.clone();
    let item_id_for_async = item_id.clone();
    let label_models: Vec<todos::entity::LabelModel> = {
        let store = cx.global::<TodoStore>();
        item_for_save
            .labels
            .as_deref()
            .unwrap_or("")
            .split(';')
            .filter(|s| !s.is_empty())
            .filter_map(|id| store.get_label(id).map(|l| l.as_ref().clone()))
            .collect()
    };

    cx.spawn(async move |cx| {
        let spawn_start = std::time::Instant::now();
        let save_result = db_state
            .spawn_store_op(move |store| async move {
                retry::retry_async_todo(
                    |_attempt| {
                        let store = store.clone();
                        let item = item_for_save.clone();
                        async move { store.insert_item(item.as_ref().clone(), true).await }
                    },
                    RetryConfig::for_db_operation(),
                )
                .await
            })
            .await;

        debug!(
            "add_item_optimistic finished id={} elapsed_ms={}",
            item_id_for_async,
            spawn_start.elapsed().as_millis()
        );

        match save_result {
            Ok(Ok(saved_item)) => {
                debug!("Successfully saved item with ID {}", saved_item.id);

                // ID 已在插入前确定；若数据库回写了相同记录，仍同步一次内存态
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(saved_item.clone()));
                });

                if !label_models.is_empty() {
                    let item_id = saved_item.id.clone();
                    match db_state_for_labels
                        .spawn_store_op(move |store| async move {
                            store.set_item_labels_from_models(&item_id, &label_models).await
                        })
                        .await
                    {
                        Ok(Ok(())) => {
                            debug!("Labels saved for new item {}", saved_item.id);
                        },
                        Ok(Err(e)) => {
                            error!("❌ 新建任务后写入标签失败: {}", e);
                        },
                        Err(e) => {
                            error!("❌ 新建任务标签任务异常: {:?}", e);
                        },
                    }
                }

                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_succeeded(saved_item.id);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "add_item_optimistic",
                    &item_id_for_error,
                );
                error!("❌ 添加任务失败（重试耗尽）: {}", context.format_user_message());

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(
                        t!(
                            "todo.error.add_failed",
                            error => context.format_user_message()
                        )
                        .to_string(),
                    );
                });

                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id_for_async.clone());
                });
            },
            Err(join_err) => {
                error!("❌ 添加任务异常（任务被取消或 panic）: {:?}", join_err);

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(t!("todo.error.add_internal").to_string());
                });
            },
        }
    })
    .detach();

    item_id
}

/// 乐观更新任务：先写 TodoStore，再在 DB runtime 中落盘。
pub fn update_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "update_item_optimistic");
        error!("{}", context.format_user_message());
        cx.update_global::<ErrorNotifier, _>(|notifier, _| {
            notifier.set_error(context.format_user_message());
        });
        return;
    }

    let before = cx.global::<TodoStore>().get_item(&item.id);
    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(item.clone());
    });
    if let Some(before) = before {
        cx.update_global::<UndoStack, _>(|stack, _| {
            stack.record(UndoEntry::Updated { before, after: item.clone() });
        });
    }

    let item_id = item.id.clone();
    let item_for_db = item.clone();
    let db_state = cx.global::<DBState>().clone();

    cx.spawn(async move |cx| {
        let save_result = db_state
            .spawn_store_op(move |store| async move {
                retry::retry_async_todo(
                    |_attempt| {
                        let store = store.clone();
                        let item = item_for_db.clone();
                        async move { store.update_item(item.as_ref().clone(), "").await }
                    },
                    RetryConfig::for_db_operation(),
                )
                .await
            })
            .await;

        match save_result {
            Ok(Ok(updated_item)) => {
                debug!(
                    "Successfully saved item update: {} with priority: {:?}, content: '{}'",
                    item_id, updated_item.priority, updated_item.content
                );
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_succeeded(item_id);
                });
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "update_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());
                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(
                        t!(
                            "todo.error.update_failed",
                            error => context.format_user_message()
                        )
                        .to_string(),
                    );
                });
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id);
                });
            },
            Err(join_err) => {
                error!("Item update task panicked: {:?}", join_err);
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    results.mark_failed(item_id);
                });
            },
        }
    })
    .detach();
}

/// 乐观删除任务
pub fn delete_item_optimistic(item: Arc<ItemModel>, cx: &mut App) {
    let item_id = item.id.clone();

    debug!("Optimistically deleting item: {}", item_id);

    cx.update_global::<UndoStack, _>(|stack, _| {
        stack.record(UndoEntry::Deleted(item.clone()));
    });

    cx.update_global::<TodoStore, _>(|store, _| {
        store.remove_item(&item_id);
    });

    let item_for_recovery = item.clone();
    let db_state = cx.global::<DBState>().clone();
    let item_id_for_db = item_id.clone();

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.delete_item(&item_id_for_db).await })
            .await
        {
            Ok(Ok(_)) => {
                debug!("Successfully deleted item from database: {}", item_id);
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "delete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(item_for_recovery.clone());
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(
                        t!(
                            "todo.error.delete_failed",
                            error => context.format_user_message()
                        )
                        .to_string(),
                    );
                });
            },
            Err(join_err) => {
                error!("Item delete task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

/// 乐观设置置顶状态
pub fn set_item_pinned_optimistic(item: Arc<ItemModel>, pinned: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let old_pinned = item.pinned;

    debug!("Optimistically {} item: {}", if pinned { "pinning" } else { "unpinning" }, item_id);

    let mut updated_item = (*item).clone();
    updated_item.pinned = pinned;
    let after = Arc::new(updated_item.clone());

    cx.update_global::<UndoStack, _>(|stack, _| {
        stack.record(UndoEntry::Updated { before: item.clone(), after: after.clone() });
    });

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(after);
    });

    let db_state = cx.global::<DBState>().clone();
    let item_id_clone = item_id.clone();

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.update_item_pin(&item_id_clone, pinned).await
            })
            .await
        {
            Ok(Ok(_)) => {
                debug!("Successfully saved pinned status: {}", item_id);
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "set_item_pinned_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                let mut reverted_item = updated_item.clone();
                reverted_item.pinned = old_pinned;
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(reverted_item));
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    let error = context.format_user_message();
                    let msg = if pinned {
                        t!("todo.error.pin_failed", error => error).to_string()
                    } else {
                        t!("todo.error.unpin_failed", error => error).to_string()
                    };
                    notifier.set_error(msg);
                });
            },
            Err(join_err) => {
                error!("Item pin task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

/// 折叠/展开子任务列表（不写入撤销栈）
pub fn set_item_collapsed_optimistic(item: Arc<ItemModel>, collapsed: bool, cx: &mut App) {
    if item.collapsed == collapsed {
        return;
    }

    let item_id = item.id.clone();
    let old_collapsed = item.collapsed;
    let mut updated_item = (*item).clone();
    updated_item.collapsed = collapsed;
    let after = Arc::new(updated_item.clone());

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(after.clone());
    });

    let db_state = cx.global::<DBState>().clone();
    let item_for_db = after;

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.update_item(item_for_db.as_ref().clone(), "").await
            })
            .await
        {
            Ok(Ok(_)) => {
                debug!("Successfully saved collapsed status: {}", item_id);
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "set_item_collapsed_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                let mut reverted_item = updated_item.clone();
                reverted_item.collapsed = old_collapsed;
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(Arc::new(reverted_item));
                });
                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    notifier.set_error(
                        t!(
                            "todo.error.toggle_subtasks",
                            error => context.format_user_message()
                        )
                        .to_string(),
                    );
                });
            },
            Err(join_err) => {
                error!("Item collapsed task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

/// 乐观完成任务
pub fn complete_item_optimistic(item: Arc<ItemModel>, checked: bool, cx: &mut App) {
    let item_id = item.id.clone();
    let original = item.clone();

    debug!(
        "Optimistically {} item: {}",
        if checked { "completing" } else { "uncompleting" },
        item_id
    );

    let mut updated_item = (*item).clone();
    let next_due = checked
        .then(|| updated_item.due_date().and_then(|d| d.next_due_after_completion()))
        .flatten();
    let rolling = next_due.is_some();
    if let Some(next_due) = next_due {
        updated_item.set_due_date(Some(next_due));
        updated_item.checked = false;
        updated_item.completed_at = None;
    } else {
        updated_item.checked = checked;
        updated_item.completed_at =
            if checked { Some(chrono::Utc::now().naive_utc()) } else { None };
    }

    let complete_subitems = !rolling;
    let children: Vec<Arc<ItemModel>> =
        if complete_subitems { cx.global::<TodoStore>().child_items(&item_id) } else { Vec::new() };
    let original_children = children.clone();
    let after_parent = Arc::new(updated_item.clone());
    let mut undo_parts =
        vec![UndoEntry::Updated { before: original.clone(), after: after_parent.clone() }];
    let mut child_afters: Vec<Arc<ItemModel>> = Vec::new();
    if complete_subitems {
        for child in &children {
            let mut child_item = (**child).clone();
            child_item.checked = checked;
            child_item.completed_at =
                if checked { Some(chrono::Utc::now().naive_utc()) } else { None };
            let after = Arc::new(child_item);
            undo_parts.push(UndoEntry::Updated { before: child.clone(), after: after.clone() });
            child_afters.push(after);
        }
    }
    cx.update_global::<UndoStack, _>(|stack, _| {
        if undo_parts.len() == 1 {
            stack.record(undo_parts.pop().unwrap());
        } else {
            stack.record(UndoEntry::Batch(undo_parts));
        }
    });

    cx.update_global::<TodoStore, _>(|store, _| {
        store.update_item(after_parent);
        for after in child_afters {
            store.update_item(after);
        }
    });

    if checked && cx.global::<crate::todo_state::TodoPrefs>().complete_sound {
        let _ = crate::play_ogg_file("assets/sounds/success.ogg");
    }

    let db_state = cx.global::<DBState>().clone();
    let item_id_for_db = item_id.clone();

    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move {
                store.complete_item(&item_id_for_db, checked, complete_subitems).await?;
                Ok(store.get_item(&item_id_for_db).await)
            })
            .await
        {
            Ok(Ok(fresh)) => {
                debug!("Successfully saved completion status: {}", item_id);
                if let Some(fresh) = fresh {
                    cx.update_global::<TodoStore, _>(|todo_store, _| {
                        todo_store.update_item(Arc::new(fresh));
                    });
                }
            },
            Ok(Err(e)) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(Box::new(e)),
                    "complete_item_optimistic",
                    &item_id,
                );
                error!("{}", context.format_user_message());

                cx.update_global::<TodoStore, _>(|store, _| {
                    store.update_item(original);
                    for child in original_children {
                        store.update_item(child);
                    }
                });

                cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                    let error = context.format_user_message();
                    let msg = if checked {
                        t!("todo.error.complete_failed", error => error).to_string()
                    } else {
                        t!("todo.error.uncomplete_failed", error => error).to_string()
                    };
                    notifier.set_error(msg);
                });
            },
            Err(join_err) => {
                error!("Item complete task panicked: {:?}", join_err);
            },
        }
    })
    .detach();
}

/// 撤销最近一次任务操作（可连续撤销）。
pub fn undo_last_task(cx: &mut App) -> Option<String> {
    apply_history(true, cx)
}

/// 重做最近一次被撤销的任务操作。
pub fn redo_last_task(cx: &mut App) -> Option<String> {
    apply_history(false, cx)
}

fn apply_history(undo: bool, cx: &mut App) -> Option<String> {
    let entry = cx.update_global::<UndoStack, _>(|stack, _| {
        stack.restoring = true;
        if undo { stack.pop_undo() } else { stack.pop_redo() }
    })?;
    apply_entry(&entry, undo, cx);
    cx.update_global::<UndoStack, _>(|stack, _| {
        stack.restoring = false;
        if undo {
            stack.push_redo(entry);
        } else {
            stack.push_undo_silent(entry);
        }
    });
    Some(if undo {
        t!("todo.notify.undone").to_string()
    } else {
        t!("todo.notify.redone").to_string()
    })
}

fn apply_entry(entry: &UndoEntry, undo: bool, cx: &mut App) {
    match entry {
        UndoEntry::Created(item) => {
            if undo {
                delete_item_optimistic(item.clone(), cx);
            } else {
                add_item_optimistic(item.clone(), cx);
            }
        },
        UndoEntry::Deleted(item) => {
            if undo {
                add_item_optimistic(item.clone(), cx);
            } else {
                delete_item_optimistic(item.clone(), cx);
            }
        },
        UndoEntry::Updated { before, after } => {
            let target = if undo { before } else { after };
            update_item_optimistic(target.clone(), cx);
        },
        UndoEntry::Batch(entries) => {
            if undo {
                for part in entries.iter().rev() {
                    apply_entry(part, true, cx);
                }
            } else {
                for part in entries {
                    apply_entry(part, false, cx);
                }
            }
        },
    }
}
