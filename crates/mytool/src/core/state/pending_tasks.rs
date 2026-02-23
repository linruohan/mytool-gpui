//! 待处理任务状态管理
//!
//! 用于跟踪正在进行的异步数据库操作，确保应用关闭前所有数据都已保存。

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use gpui::Global;
use tracing::info;

use super::events::SaveStatus;

/// 待处理任务状态
///
/// 跟踪正在进行的异步数据库操作，用于：
/// 1. 在应用关闭前确保所有数据已保存
/// 2. 在 UI 上显示保存状态
/// 3. 检测是否有未保存的更改
pub struct PendingTasksState {
    /// 正在进行的任务数量
    pending_count: AtomicUsize,
    /// 是否有错误
    has_error: AtomicBool,
    /// 最后一次错误信息
    last_error: RwLock<Option<String>>,
    /// 任务描述（用于调试）
    task_descriptions: RwLock<Vec<String>>,
}

impl Default for PendingTasksState {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingTasksState {
    pub fn new() -> Self {
        Self {
            pending_count: AtomicUsize::new(0),
            has_error: AtomicBool::new(false),
            last_error: RwLock::new(None),
            task_descriptions: RwLock::new(Vec::new()),
        }
    }

    /// 开始一个新任务（增加计数）
    pub fn start_task(&self, description: &str) {
        let count = self.pending_count.fetch_add(1, Ordering::SeqCst);
        info!("🔄 Pending task started: {} (total: {})", description, count + 1);

        if let Ok(mut descs) = self.task_descriptions.write() {
            descs.push(description.to_string());
        }
    }

    /// 完成一个任务（减少计数）
    pub fn end_task(&self, description: &str) {
        let count = self.pending_count.fetch_sub(1, Ordering::SeqCst);
        info!(
            "✅ Pending task completed: {} (remaining: {})",
            description,
            count.saturating_sub(1)
        );

        if let Ok(mut descriptions) = self.task_descriptions.write()
            && let Some(pos) = descriptions.iter().position(|d| d == description)
        {
            descriptions.remove(pos);
        }
    }

    /// 手动增加任务计数（别名，与 start_task 相同）
    pub fn increment(&self, description: &str) {
        self.start_task(description);
    }

    /// 手动减少任务计数（别名，与 end_task 相同）
    pub fn decrement(&self, description: &str) {
        self.end_task(description);
    }

    /// 获取当前待处理任务数量
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }

    /// 检查是否有待处理的任务
    pub fn has_pending_tasks(&self) -> bool {
        self.pending_count.load(Ordering::SeqCst) > 0
    }

    /// 获取当前保存状态
    pub fn save_status(&self) -> SaveStatus {
        if self.has_error.load(Ordering::SeqCst) {
            SaveStatus::HasError
        } else if self.pending_count.load(Ordering::SeqCst) > 0 {
            SaveStatus::Saving
        } else {
            SaveStatus::Idle
        }
    }

    /// 设置错误状态
    pub fn set_error(&self, error: String) {
        self.has_error.store(true, Ordering::SeqCst);
        if let Ok(mut last_error) = self.last_error.write() {
            *last_error = Some(error);
        }
    }

    /// 清除错误状态
    pub fn clear_error(&self) {
        self.has_error.store(false, Ordering::SeqCst);
        if let Ok(mut last_error) = self.last_error.write() {
            *last_error = None;
        }
    }

    /// 获取最后一次错误
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().ok()?.clone()
    }

    /// 获取当前任务描述列表
    pub fn task_descriptions(&self) -> Vec<String> {
        self.task_descriptions.read().map(|d| d.clone()).unwrap_or_default()
    }
}

impl Clone for PendingTasksState {
    fn clone(&self) -> Self {
        Self {
            pending_count: AtomicUsize::new(self.pending_count.load(Ordering::SeqCst)),
            has_error: AtomicBool::new(self.has_error.load(Ordering::SeqCst)),
            last_error: RwLock::new(self.last_error()),
            task_descriptions: RwLock::new(self.task_descriptions()),
        }
    }
}

impl Global for PendingTasksState {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_counting() {
        let state = PendingTasksState::new();

        assert_eq!(state.pending_count(), 0);
        assert!(!state.has_pending_tasks());

        state.start_task("task1");
        assert_eq!(state.pending_count(), 1);

        state.start_task("task2");
        assert_eq!(state.pending_count(), 2);

        state.end_task("task1");
        assert_eq!(state.pending_count(), 1);

        state.end_task("task2");
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_manual_counting() {
        let state = PendingTasksState::new();

        state.increment("manual1");
        assert_eq!(state.pending_count(), 1);

        state.increment("manual2");
        assert_eq!(state.pending_count(), 2);

        state.decrement("manual1");
        assert_eq!(state.pending_count(), 1);

        state.decrement("manual2");
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_error_state() {
        let state = PendingTasksState::new();

        assert_eq!(state.save_status(), SaveStatus::Idle);

        state.set_error("Test error".to_string());
        assert_eq!(state.save_status(), SaveStatus::HasError);
        assert_eq!(state.last_error(), Some("Test error".to_string()));

        state.clear_error();
        assert_eq!(state.save_status(), SaveStatus::Idle);
    }
}
