use std::sync::Arc;

use gpui::{Context, Entity, Window};
use todos::entity::LabelModel;
use tracing::info;

use crate::{
    LabelsPopoverEvent, LabelsPopoverList,
    core::notification::NotificationSystem,
};

use super::{ItemInfoEvent, ItemInfoState};

impl ItemInfoState {
    pub fn on_labels_event(
        &mut self,
        _state: &Entity<LabelsPopoverList>,
        event: &LabelsPopoverEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            LabelsPopoverEvent::Selected(label) => {
                let label_model = (**label).clone();
                self.add_checked_labels(Arc::new(label_model), window, cx);
                self.sync_labels_field_from_selection(cx);
            },
            LabelsPopoverEvent::DeSelected(label) => {
                let label_model = (**label).clone();
                self.rm_checked_labels(Arc::new(label_model), window, cx);
                self.sync_labels_field_from_selection(cx);
            },
            LabelsPopoverEvent::LabelsChanged(label_ids) => {
                info!(
                    "on_labels_event: LabelsChanged - item_id: {}, label_ids: '{}'",
                    self.state_manager.item.id, label_ids
                );

                // 🚀 关键修复：只更新本地状态，不立即保存
                // 标签保存会在 save_all_changes 中统一处理，避免并发写锁竞争
                self.state_manager.update_item(|item| {
                    item.labels = Some(label_ids.clone());
                });

                // 更新 LabelsPopoverList 的选中状态
                self.label_popover_list.update(cx, |popover_list, cx| {
                    popover_list.set_item_checked_label_id_async(label_ids.clone(), cx);
                });

                // 发送事件通知 UI 更新
                cx.emit(ItemInfoEvent::Updated());
                cx.notify();
            },
        }
    }

    fn sync_labels_field_from_selection(&mut self, cx: &mut Context<Self>) {
        let selected_label_ids = self
            .selected_labels(cx)
            .iter()
            .map(|l| l.id.clone())
            .collect::<Vec<_>>()
            .join(";")
            .to_string();
        self.state_manager.update_item(|item| {
            item.labels = Some(selected_label_ids);
        });
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn add_checked_labels(
        &mut self,
        label: Arc<LabelModel>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_id = self.state_manager.item.id.clone();
        let label_name = label.name.clone();

        // 先更新本地状态，确保UI立即响应且状态保持一致
        self.label_popover_list.update(cx, |popover_list, cx| {
            if !popover_list.selected_labels.iter().any(|l| l.id == label.id) {
                popover_list.selected_labels.push(label.clone());
                // 同步更新 LabelCheckListDelegate 的 checked_list
                popover_list.label_list.update(cx, |list, cx| {
                    list.delegate_mut()
                        .set_item_checked_labels(popover_list.selected_labels.clone(), cx);
                });
            }
        });

        // 使用全局 Store 持久化标签变更
        let db_state = cx.global::<crate::todo_state::DBState>().clone();

        cx.spawn(async move |_this, _cx| {
            if !db_state.is_store_ready() {
                tracing::warn!("Store not ready, skipping add_label_to_item");
                return;
            }
            let store = db_state.get_store_async().await;
            match store.add_label_to_item(&item_id, &label_name).await {
                Ok(_) => {
                    NotificationSystem::debug(format!("Label '{}' added to item", label_name));
                },
                Err(e) => {
                    NotificationSystem::log_error("Failed to add label to item", e);
                },
            }
        })
        .detach();
    }

    pub fn rm_checked_labels(
        &mut self,
        label: Arc<LabelModel>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_id = self.state_manager.item.id.clone();
        let label_id = label.id.clone();

        // 先更新本地状态，确保 UI 立即响应且状态保持一致
        self.label_popover_list.update(cx, |popover_list, cx| {
            popover_list.selected_labels.retain(|l| l.id != label.id);
            // 同步更新 LabelCheckListDelegate 的 checked_list
            popover_list.label_list.update(cx, |list, cx| {
                list.delegate_mut()
                    .set_item_checked_labels(popover_list.selected_labels.clone(), cx);
            });
        });

        // 使用全局 Store 持久化标签变更
        let db_state = cx.global::<crate::todo_state::DBState>().clone();

        cx.spawn(async move |_this, _cx| {
            if !db_state.is_store_ready() {
                tracing::warn!("Store not ready, skipping remove_label_from_item");
                return;
            }
            let store = db_state.get_store_async().await;
            match store.remove_label_from_item(&item_id, &label_id).await {
                Ok(_) => {
                    NotificationSystem::debug("Label removed from item");
                },
                Err(e) => {
                    NotificationSystem::log_error("Failed to remove label from item", e);
                },
            }
        })
        .detach();
    }

    /// 获取选中的 Labels
    ///
    /// 注意：由于 Labels 现在存储在关联表中，此方法返回的是本地缓存的 labels
    /// 如果需要最新的 labels，请使用异步方法从数据库加载
    pub fn selected_labels(&self, cx: &mut Context<Self>) -> Vec<Arc<LabelModel>> {
        // 从 LabelPopoverList 获取当前选中的 labels
        self.label_popover_list.read(cx).selected_labels.clone()
    }

    /// 从当前 item 的 labels 字段刷新 LabelsPopoverList 的选中状态
    /// 用于在外部标签更新后同步 UI 状态
    pub fn refresh_labels_selection_from_item(&mut self, cx: &mut Context<Self>) {
        let item_labels_str = self.state_manager.item.labels.clone().unwrap_or_default();

        // 更新 LabelsPopoverList 的选中状态
        self.label_popover_list.update(cx, |popover_list, cx| {
            popover_list.set_item_checked_label_id_async(item_labels_str.clone(), cx);
        });

        // 通知 ItemInfoState 更新
        cx.notify();
    }

    // label_toggle_checked：label选中或取消选中
    pub(super) fn label_toggle_checked(
        &mut self,
        label: Arc<LabelModel>,
        selected: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("Label toggle clicked: {} -> {}", label.name, selected);

        // 先更新 label_popover_list 的状态，确保两个UI保持同步
        self.label_popover_list.update(cx, |popover_list, cx| {
            if *selected {
                // 添加到选中列表
                if !popover_list.selected_labels.iter().any(|l| l.id == label.id) {
                    popover_list.selected_labels.push(label.clone());
                }
            } else {
                // 从选中列表移除
                popover_list.selected_labels.retain(|l| l.id != label.id);
            }
            // 同步更新 LabelCheckListDelegate 的 checked_list
            popover_list.label_list.update(cx, |list, cx| {
                list.delegate_mut()
                    .set_item_checked_labels(popover_list.selected_labels.clone(), cx);
            });
        });

        // 更新 state_manager.item.labels 字段
        let selected_label_ids = self
            .selected_labels(cx)
            .iter()
            .map(|l| l.id.clone())
            .collect::<Vec<_>>()
            .join(";")
            .to_string();
        info!(
            "label_toggle_checked: updating labels - item_id: {}, selected_label_ids: '{}'",
            self.state_manager.item.id, selected_label_ids
        );
        self.state_manager.update_item(|item| {
            item.labels = Some(selected_label_ids.clone());
        });
        info!(
            "label_toggle_checked: updated labels - item_id: {}, labels: {:?}",
            self.state_manager.item.id, self.state_manager.item.labels
        );

        // 持久化到数据库
        let item_id = self.state_manager.item.id.clone();
        let label_ids_vec: Vec<String> = selected_label_ids
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // 使用全局 Store 持久化标签变更
        let db_state = cx.global::<crate::todo_state::DBState>().clone();

        cx.spawn(async move |_this, _cx| {
            if !db_state.is_store_ready() {
                tracing::warn!("Store not ready, skipping set_item_labels");
                return;
            }
            let store = db_state.get_store_async().await;
            match store.set_item_labels(&item_id, &label_ids_vec).await {
                Ok(_) => {
                    NotificationSystem::debug(format!(
                        "Labels updated for item {}: {:?}",
                        item_id, label_ids_vec
                    ));
                },
                Err(e) => {
                    NotificationSystem::log_error("Failed to set item labels", e);
                },
            }
        })
        .detach();

        // 不跳过更新，让 update_item_optimistic 更新 TodoStore
        // 触发UI更新
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }
}
