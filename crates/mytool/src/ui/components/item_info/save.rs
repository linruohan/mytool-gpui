use gpui::Context;
use tracing::{error, info, warn};

use super::{ItemInfoEvent, ItemInfoState, SaveItemStatus};
use crate::{
    core::state::TodoStore,
    state_service,
    todo_actions::{add_item_optimistic, complete_item_optimistic, delete_item_optimistic},
};

impl ItemInfoState {
    pub fn sync_inputs(&mut self, cx: &mut Context<Self>) -> bool {
        let name = self.name_input.read(cx).value().to_string();
        let desc = self.desc_input.read(cx).value().to_string();
        let new_desc = if desc.is_empty() { None } else { Some(desc) };

        let current_item = &self.state_manager.item;
        let changed = current_item.content != name || current_item.description != new_desc;
        if changed {
            self.state_manager.set_content(name);
            self.state_manager.set_description(new_desc);
        }
        changed
    }

    /// 保存所有修改到数据库
    pub fn save_all_changes(&mut self, cx: &mut Context<Self>) {
        // 🚨 添加明显的日志标记，方便调试
        tracing::debug!("save_all_changes START - item_id: {}", self.state_manager.item.id);
        info!("🔔🔔🔔 save_all_changes START - item_id: {}", self.state_manager.item.id);

        // 同步输入框内容
        let has_input_changes = self.sync_inputs(cx);

        // 先克隆需要的数据，避免借用冲突
        let current_item = self.state_manager.item.clone();
        let item_id = current_item.id.clone();
        let item_labels_str = current_item.labels.clone().unwrap_or_default();

        // 获取当前选中的标签
        let selected_label_ids: Vec<String> =
            self.selected_labels(cx).iter().map(|l| l.id.clone()).collect();
        let new_labels_str = selected_label_ids.join(";");

        let labels_changed = item_labels_str != new_labels_str;

        // 🚀 关键修复：检查是否有未保存的修改（使用 dirty 标志）
        let has_unsaved_changes = self.state_manager.is_dirty();

        // 🚀 关键修复：根据任务是否有 ID 来决定是添加还是更新
        info!(
            "save_all_changes called for item: {}, has_input_changes: {}, has_unsaved_changes: \
             {}, content: '{}', labels_changed: {}",
            item_id, has_input_changes, has_unsaved_changes, current_item.content, labels_changed
        );

        // 如果没有修改，直接跳过保存（但新任务除外）
        // 🔧 修复：新任务（item_id 为空）即使没有检测到修改也应该保存
        if !has_input_changes && !labels_changed && !has_unsaved_changes && !item_id.is_empty() {
            info!(
                "save_all_changes: No changes detected for item {}, skipping database update",
                item_id
            );
            return;
        }

        // 根据 item.id 是否为空来决定是添加新任务还是更新现有任务
        if item_id.is_empty() {
            // 新建任务：使用 add_item_optimistic
            // 🚀 关键修复：设置跳过下一次更新标志，避免 TodoStore 更新触发的观察者回调导致死锁
            self.state_manager.skip_next_update = true;
            info!(
                "Triggering add_item_optimistic for new item with content: '{}'",
                current_item.content
            );
            let temp_id = add_item_optimistic(current_item.clone(), cx);

            // 更新原始 item 对象的 ID 为临时 ID
            if !temp_id.is_empty() {
                info!("Updating original item ID to temp ID: {}", temp_id);
                let temp_id_clone = temp_id.clone();
                self.state_manager.update_item(|item| {
                    item.id = temp_id_clone.clone();
                });
            }

            // 🚀 7.0修复：直接标记已保存，不通过事件触发，避免阻塞
            // cx.emit(ItemInfoEvent::Added());
            info!("save_all_changes: marking clean for new item without event");
            self.state_manager.update_original();
        } else {
            // 🚀 关键修复：统一保存所有修改，包括标签
            info!(
                "save_all_changes: item={}, labels_changed={}, old_labels='{}', new_labels='{}'",
                item_id, labels_changed, item_labels_str, new_labels_str
            );

            // 如果标签发生变化，使用异步保存标签（不阻塞UI）
            if labels_changed {
                info!("save_all_changes: saving labels for item {}", item_id);
                let label_ids_to_save = selected_label_ids.clone();
                let item_id_for_labels = item_id.clone();

                // 🚀 关键优化：先更新本地状态（乐观更新），UI立即响应
                self.state_manager.update_item(|item| {
                    item.labels = Some(new_labels_str.clone());
                });

                // 获取db_state用于异步任务
                let db_state = cx.global::<crate::todo_state::DBState>().clone();

                // ✅ 修复：使用 cx.spawn 异步保存标签，不阻塞UI线程
                cx.spawn(async move |_this, cx| {
                    let item_id_for_log = item_id_for_labels.clone(); // 用于日志
                    tracing::debug!("Executing async label save for item: {}", item_id_for_labels);
                    match crate::core::tokio_runtime::spawn_db_operation(async move {
                        // 🚀 7.0修复：等待 Store 就绪，而非静默跳过
                        db_state
                            .wait_for_store_ready(Some(std::time::Duration::from_secs(5)))
                            .await?;
                        let store = db_state.get_store_async().await;
                        store.set_item_labels(&item_id_for_labels, &label_ids_to_save).await
                    })
                    .await
                    {
                        Ok(result) => match result {
                            Ok(_) => {
                                info!(
                                    "save_all_changes: labels saved successfully for item {}",
                                    item_id_for_log
                                );
                            },
                            Err(e) => {
                                error!(
                                    "save_all_changes: failed to save labels for item {}: {:?}",
                                    item_id_for_log, e
                                );
                                cx.update_global::<crate::core::state::ErrorNotifier, _>(
                                    |notifier, _| {
                                        notifier.set_error(format!(
                                            "标签保存失败：{}，请检查网络连接后重试",
                                            e
                                        ));
                                    },
                                );
                            },
                        },
                        Err(e) => {
                            error!("save_all_changes: label save task panicked: {:?}", e);
                        },
                    }
                })
                .detach();

                tracing::debug!("Label save dispatched asynchronously for item: {}", item_id);
            }

            tracing::debug!("Preparing async item save for item: {}", item_id);
            let db_state = cx.global::<crate::todo_state::DBState>().clone();
            let item_for_save = self.state_manager.item.clone();
            let item_id_for_save = item_id.clone();

            // ✅ 修复：使用 cx.spawn 异步保存item内容，不阻塞UI线程
            cx.spawn(async move |_this, cx| {
                let item_id_for_log = item_id_for_save.clone(); // 用于日志和事件
                let item_for_update = item_for_save.clone(); // 用于更新TodoStore
                tracing::debug!("Executing async database save for item: {}", item_id_for_save);
                let save_result = crate::core::tokio_runtime::spawn_db_operation(async move {
                    // 🚀 7.0修复：等待 Store 就绪，而非静默跳过
                    db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(5))).await?;
                    let store = db_state.get_store_async().await;

                    // 🚀 7.0新增：使用重试机制，自动处理临时性错误
                    crate::core::utils::retry::retry_async_todo(
                        move |_attempt| {
                            let store = store.clone();
                            let item = item_for_save.clone();
                            async move { state_service::mod_item_with_store(item, store).await }
                        },
                        crate::core::utils::retry::RetryConfig::for_db_operation(),
                    )
                    .await
                })
                .await;

                match save_result {
                    Ok(result) => match result {
                        Ok(_updated_item) => {
                            tracing::info!("Item saved successfully: {}", item_id_for_log);
                            // 仅在成功时更新 TodoStore 和发布事件
                            cx.update_global::<TodoStore, _>(|store, _| {
                                store.update_item(item_for_update.clone());
                            });
                            // 🚀 7.0修复：记录保存成功结果，让主线程后续处理
                            cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                                results.mark_succeeded(item_id_for_log.clone());
                            });
                        },
                        Err(e) => {
                            tracing::error!("Failed to save item {}: {:?}", item_id_for_log, e);
                            // 🚀 7.0修复：失败时通知用户并记录失败结果
                            cx.update_global::<crate::core::state::ErrorNotifier, _>(
                                |notifier, _| {
                                    notifier
                                        .set_error(format!("任务保存失败：{}，请检查后重试", e));
                                },
                            );
                            // 🚀 7.0修复：记录保存失败结果
                            cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                                results.mark_failed(item_id_for_log.clone());
                            });
                        },
                    },
                    Err(e) => {
                        tracing::error!("Item save task panicked: {:?}", e);
                    },
                }
            })
            .detach();

            tracing::debug!("Async database save dispatched for item: {}", item_id);

            // UI立即响应：发布事件（但不立即 mark_clean）
            self.state_manager.save_status = SaveItemStatus::Saving;
            cx.emit(ItemInfoEvent::Updated());
            // 🚀 7.0修复：不再立即 mark_clean()
            // 改为等待异步任务完成后，通过 SaveResults 机制处理
            info!(
                "save_all_changes: async save dispatched for item {}, waiting for result",
                item_id
            );
        }
    }

    pub fn handle_item_info_event(&mut self, event: &ItemInfoEvent, cx: &mut Context<Self>) {
        match event {
            ItemInfoEvent::Finished() => {
                complete_item_optimistic(self.state_manager.item.clone(), true, cx);
            },
            ItemInfoEvent::Added() => {
                info!("Handling Added event for item: {}", self.state_manager.item.id);
                self.state_manager.update_original();
            },
            ItemInfoEvent::Updated() => {
                info!("Handling Updated event for item: {}", self.state_manager.item.id);
                self.state_manager.skip_next_update = false;
                self.state_manager.update_original();
            },
            ItemInfoEvent::Deleted() => {
                delete_item_optimistic(self.state_manager.item.clone(), cx);
            },
            ItemInfoEvent::UnFinished() => {
                complete_item_optimistic(self.state_manager.item.clone(), false, cx);
            },
            ItemInfoEvent::Cancelled() => {
                info!("Handling Cancelled event for item: {}", self.state_manager.item.id);
                self.cancel_edit(cx);
            },
            // 🚀 7.0修复：异步保存成功后才标记为已保存
            ItemInfoEvent::SaveSucceeded() => {
                info!(
                    "Handling SaveSucceeded event, marking clean: {}",
                    self.state_manager.item.id
                );
                self.state_manager.mark_clean();
                self.state_manager.update_original();
            },
            // 🚀 7.0修复：异步保存失败时恢复脏标记，允许重新保存
            ItemInfoEvent::SaveFailed() => {
                warn!("Handling SaveFailed event, keeping dirty: {}", self.state_manager.item.id);
                self.state_manager.mark_dirty();
            },
        }
        cx.notify();
    }

    /// 取消编辑，恢复原始数据
    pub fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        let was_new = self.state_manager.is_new_item();

        // 恢复到原始数据
        self.state_manager.revert_to_original();

        // 如果是新建任务，通知父组件删除这个临时项
        if was_new {
            cx.emit(ItemInfoEvent::Deleted());
        }

        cx.notify();
    }
}
