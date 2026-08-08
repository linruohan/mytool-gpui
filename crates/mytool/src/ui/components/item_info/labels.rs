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

    /// label 行内 checkbox：选中或取消选中
    pub(super) fn label_toggle_checked(
        &mut self,
        label: Arc<LabelModel>,
        selected: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_label_selection(label, *selected, true, window, cx);
    }

    /// 统一标签选择路径：更新本地选中 → 同步 item.labels → 可选持久化
    fn apply_label_selection(
        &mut self,
        label: Arc<LabelModel>,
        selected: bool,
        persist: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("Label selection: {} -> {}", label.name, selected);

        self.label_popover_list.update(cx, |popover_list, cx| {
            if selected {
                if !popover_list.selected_labels.iter().any(|l| l.id == label.id) {
                    popover_list.selected_labels.push(label.clone());
                }
            } else {
                popover_list.selected_labels.retain(|l| l.id != label.id);
            }
            popover_list.label_list.update(cx, |list, cx| {
                list.delegate_mut()
                    .set_item_checked_labels(popover_list.selected_labels.clone(), cx);
            });
        });

        let selected_label_ids = self
            .selected_labels(cx)
            .iter()
            .map(|l| l.id.clone())
            .collect::<Vec<_>>()
            .join(";");
        self.state_manager.update_item(|item| {
            item.labels = Some(selected_label_ids.clone());
        });

        if persist {
            self.persist_item_labels(&selected_label_ids, cx);
        }

        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    fn persist_item_labels(&self, selected_label_ids: &str, cx: &mut Context<Self>) {
        let item_id = self.state_manager.item.id.clone();
        if item_id.is_empty() || item_id.starts_with("temp_") {
            return;
        }

        let label_ids_vec: Vec<String> = selected_label_ids
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

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
    }
}
