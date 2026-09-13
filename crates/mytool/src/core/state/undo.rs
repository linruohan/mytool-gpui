//! 任务操作撤销 / 重做栈

use std::sync::Arc;

use gpui::Global;
use todos::entity::ItemModel;

const MAX_ENTRIES: usize = 40;

#[derive(Clone)]
pub enum UndoEntry {
    Created(Arc<ItemModel>),
    Deleted(Arc<ItemModel>),
    Updated { before: Arc<ItemModel>, after: Arc<ItemModel> },
    Batch(Vec<UndoEntry>),
}

#[derive(Default)]
pub struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
    pub restoring: bool,
}

impl Global for UndoStack {}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: UndoEntry) {
        if self.restoring {
            return;
        }
        if self.undo.len() >= MAX_ENTRIES {
            self.undo.remove(0);
        }
        self.undo.push(entry);
        self.redo.clear();
    }

    pub fn pop_undo(&mut self) -> Option<UndoEntry> {
        self.undo.pop()
    }

    pub fn pop_redo(&mut self) -> Option<UndoEntry> {
        self.redo.pop()
    }

    pub fn push_redo(&mut self, entry: UndoEntry) {
        self.redo.push(entry);
    }

    pub fn push_undo_silent(&mut self, entry: UndoEntry) {
        if self.undo.len() >= MAX_ENTRIES {
            self.undo.remove(0);
        }
        self.undo.push(entry);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> Arc<ItemModel> {
        Arc::new(ItemModel { id: id.into(), content: id.into(), ..Default::default() })
    }

    #[test]
    fn record_clears_redo_and_caps_size() {
        let mut stack = UndoStack::new();
        stack.redo.push(UndoEntry::Deleted(item("x")));
        for i in 0..45 {
            stack.record(UndoEntry::Created(item(&i.to_string())));
        }
        assert_eq!(stack.undo.len(), MAX_ENTRIES);
        assert!(stack.redo.is_empty());
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn restoring_skips_record() {
        let mut stack = UndoStack::new();
        stack.restoring = true;
        stack.record(UndoEntry::Created(item("a")));
        assert!(stack.undo.is_empty());
    }

    #[test]
    fn pop_undo_then_push_redo() {
        let mut stack = UndoStack::new();
        stack.record(UndoEntry::Created(item("a")));
        let entry = stack.pop_undo().unwrap();
        stack.push_redo(entry);
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        assert!(stack.pop_redo().is_some());
    }
}
