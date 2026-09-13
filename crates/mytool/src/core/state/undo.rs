//! 最近一次任务操作的撤销（完成 / 删除）

use std::sync::Arc;

use gpui::Global;
use todos::entity::ItemModel;

#[derive(Clone)]
pub enum UndoEntry {
    Deleted(Arc<ItemModel>),
    Completed { before: Arc<ItemModel> },
}

#[derive(Default)]
pub struct UndoStack {
    pub last: Option<UndoEntry>,
    pub restoring: bool,
}

impl Global for UndoStack {}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: UndoEntry) {
        if !self.restoring {
            self.last = Some(entry);
        }
    }
}
