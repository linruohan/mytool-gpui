//! 错误通知与异步保存结果追踪
//!
//! 邮箱字段用 Mutex，观察者消费时不必 `update_global`，避免再次通知自己卡死。

use std::sync::Mutex;

use gpui::Global;

/// 保存状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    /// 空闲，没有正在进行的保存操作
    Idle,
    /// 正在保存
    Saving,
    /// 保存错误
    HasError,
}

/// 错误通知器
///
/// 用于在后台任务发生错误时存储错误消息，供 UI 层显示通知
pub struct ErrorNotifier {
    last_error: Mutex<Option<String>>,
}
impl Default for ErrorNotifier {
    fn default() -> Self {
        Self::new()
    }
}
impl Global for ErrorNotifier {}
impl ErrorNotifier {
    pub fn new() -> Self {
        Self { last_error: Mutex::new(None) }
    }

    pub fn set_error(&self, message: String) {
        *self.last_error.lock().unwrap() = Some(message);
    }

    pub fn take_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().take()
    }

    pub fn peek_error(&self) -> bool {
        self.last_error.lock().unwrap().is_some()
    }
}

/// 异步保存结果追踪器
///
/// 用于记录异步保存操作的结果，让主线程能够在适当时机检查并处理。
/// 解决了异步任务无法直接调用 cx.emit() 的问题。
#[derive(Default)]
pub struct SaveResults {
    succeeded: Mutex<Vec<String>>,
    failed: Mutex<Vec<String>>,
}

impl Global for SaveResults {}
impl SaveResults {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录保存成功
    pub fn mark_succeeded(&self, item_id: String) {
        self.succeeded.lock().unwrap().push(item_id);
    }

    /// 记录保存失败
    pub fn mark_failed(&self, item_id: String) {
        self.failed.lock().unwrap().push(item_id);
    }

    /// 查看指定 item 是否已有保存结果（不消费）
    pub fn peek_result(&self, item_id: &str) -> Option<bool> {
        if self.succeeded.lock().unwrap().iter().any(|id| id == item_id) {
            Some(true)
        } else if self.failed.lock().unwrap().iter().any(|id| id == item_id) {
            Some(false)
        } else {
            None
        }
    }

    /// 检查并取出指定 item 的保存结果
    ///
    /// 返回 `Some(true)` 表示成功，`Some(false)` 表示失败，`None` 表示无结果
    pub fn take_result(&self, item_id: &str) -> Option<bool> {
        let mut succeeded = self.succeeded.lock().unwrap();
        if let Some(pos) = succeeded.iter().position(|id| id == item_id) {
            succeeded.remove(pos);
            return Some(true);
        }
        drop(succeeded);
        let mut failed = self.failed.lock().unwrap();
        if let Some(pos) = failed.iter().position(|id| id == item_id) {
            failed.remove(pos);
            return Some(false);
        }
        None
    }

    /// 清空所有结果
    pub fn clear(&self) {
        self.succeeded.lock().unwrap().clear();
        self.failed.lock().unwrap().clear();
    }
}
