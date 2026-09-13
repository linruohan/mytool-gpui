use std::sync::Arc;

use gpui::{Context, Entity, Window};
use rust_i18n::t;
use todos::entity::LabelModel;

use super::{ItemInfoEvent, ItemInfoState};
use crate::{LabelsPopoverEvent, LabelsPopoverList, core::notification::NotificationSystem};

impl ItemInfoState {
    pub fn on_labels_event(
        &mut self,
        _state: &Entity<LabelsPopoverList>,
        event: &LabelsPopoverEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            LabelsPopoverEvent::LabelsChanged(label_ids) => {
                self.state_manager.update_item(|item| {
                    item.labels = Some(label_ids.clone());
                });
                self.persist_item_labels(label_ids, cx);
                cx.emit(ItemInfoEvent::Updated());
                cx.notify();
            },
        }
    }

    /// 从当前 item 的 labels 字段刷新 LabelsPopoverList 的选中状态
    pub fn refresh_labels_selection_from_item(&mut self, cx: &mut Context<Self>) {
        let item_labels_str = self.state_manager.item.labels.clone().unwrap_or_default();
        self.label_popover_list.update(cx, |popover_list, cx| {
            popover_list.set_item_checked_label_id_async(item_labels_str, cx);
        });
        cx.notify();
    }

    /// 获取选中的 Labels（本地缓存）
    pub fn selected_labels(&self, cx: &mut Context<Self>) -> Vec<Arc<LabelModel>> {
        self.label_popover_list.read(cx).selected_labels.clone()
    }

    pub(super) fn persist_item_labels(&self, selected_label_ids: &str, cx: &mut Context<Self>) {
        let item_id = self.state_manager.item.id.clone();
        if item_id.is_empty() || item_id.starts_with("temp_") {
            return;
        }

        let label_ids_vec: Vec<String> = selected_label_ids
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let selected_labels: Vec<todos::entity::LabelModel> = {
            let store = cx.global::<crate::todo_state::TodoStore>();
            label_ids_vec
                .iter()
                .filter_map(|id| store.get_label(id).map(|l| l.as_ref().clone()))
                .collect()
        };

        let db_state = cx.global::<crate::todo_state::DBState>().clone();
        let item_id_for_log = item_id.clone();
        let label_ids_for_log = label_ids_vec.clone();
        cx.spawn(async move |_this, cx| {
            match db_state
                .spawn_store_op(move |store| async move {
                    store.set_item_labels_from_models(&item_id, &selected_labels).await
                })
                .await
            {
                Ok(Ok(_)) => {
                    NotificationSystem::debug(format!(
                        "Labels updated for item {}: {:?}",
                        item_id_for_log, label_ids_for_log
                    ));
                },
                Ok(Err(e)) => {
                    NotificationSystem::log_error("Failed to set item labels", &e);
                    cx.update_global::<crate::core::state::ErrorNotifier, _>(|notifier, _| {
                        notifier.set_error(
                            t!("todo.label.save_failed", error => e.to_string()).to_string(),
                        );
                    });
                },
                Err(e) => {
                    NotificationSystem::log_error("Label save task panicked", e);
                },
            }
        })
        .detach();
    }
}
