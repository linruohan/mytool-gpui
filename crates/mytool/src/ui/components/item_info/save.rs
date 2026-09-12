use gpui::Context;
use tracing::{info, warn};

use super::{ItemInfoEvent, ItemInfoState, SaveItemStatus};
use crate::{
    core::state::TodoStore,
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

    /// 已有任务：先挡住 TodoStore 回写，再走统一乐观更新。
    pub(super) fn persist_existing_item(&mut self, cx: &mut Context<Self>) {
        if self.state_manager.is_new_item() {
            return;
        }
        self.state_manager.skip_next_update = true;
        crate::todo_actions::update_item_optimistic(self.state_manager.item.clone(), cx);
        self.state_manager.save_status = SaveItemStatus::Saving;
    }

    /// 保存所有修改：新建走 add_item_optimistic，已有任务走 update_item_optimistic。
    pub fn save_all_changes(&mut self, cx: &mut Context<Self>) {
        tracing::debug!("save_all_changes START - item_id: {}", self.state_manager.item.id);

        let has_input_changes = self.sync_inputs(cx);

        let current_item = self.state_manager.item.clone();
        let item_id = current_item.id.clone();
        let item_labels_str = current_item.labels.clone().unwrap_or_default();

        let selected_label_ids: Vec<String> =
            self.selected_labels(cx).iter().map(|l| l.id.clone()).collect();
        let new_labels_str = selected_label_ids.join(";");
        let labels_changed = item_labels_str != new_labels_str;
        let has_unsaved_changes = self.state_manager.is_dirty();

        if !has_input_changes && !labels_changed && !has_unsaved_changes && !item_id.is_empty() {
            tracing::debug!("save_all_changes: no changes for item {}, skip", item_id);
            return;
        }

        if labels_changed {
            self.state_manager.update_item(|item| {
                item.labels = Some(new_labels_str.clone());
            });
        }

        if item_id.is_empty() {
            self.state_manager.skip_next_update = true;
            let current_item = self.state_manager.item.clone();
            let new_id = add_item_optimistic(current_item, cx);

            if new_id.is_empty() {
                warn!("save_all_changes: add_item_optimistic rejected item (validation)");
                self.state_manager.save_status = SaveItemStatus::Failed;
                self.state_manager.mark_dirty();
                cx.notify();
                return;
            }

            self.state_manager.update_item(|item| {
                item.id = new_id;
            });
            self.state_manager.update_original();
            self.state_manager.save_status = SaveItemStatus::Saving;
            return;
        }

        if item_id.starts_with("temp_") {
            let resolved = cx
                .global::<TodoStore>()
                .get_real_id(&item_id)
                .cloned()
                .unwrap_or_else(|| item_id.clone());
            if resolved != item_id {
                tracing::debug!("save_all_changes: resolving temp ID {} -> {}", item_id, resolved);
                self.state_manager.update_item(|item| {
                    item.id = resolved;
                });
            }
        }

        if labels_changed {
            self.persist_item_labels(&new_labels_str, cx);
        }

        self.persist_existing_item(cx);
        cx.emit(ItemInfoEvent::Updated());
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
                tracing::debug!("Handling Updated event for item: {}", self.state_manager.item.id);
                // 正在写入 TodoStore 时保持 skip，避免观察者立刻用库里的旧快照盖掉编辑器
                if self.state_manager.save_status != SaveItemStatus::Saving {
                    self.state_manager.skip_next_update = false;
                    self.state_manager.update_original();
                }
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
            ItemInfoEvent::SaveSucceeded() => {
                info!(
                    "Handling SaveSucceeded event, marking clean: {}",
                    self.state_manager.item.id
                );
                self.state_manager.mark_clean();
                self.state_manager.update_original();
            },
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

        self.state_manager.revert_to_original();

        if was_new {
            cx.emit(ItemInfoEvent::Deleted());
        }

        cx.notify();
    }
}
