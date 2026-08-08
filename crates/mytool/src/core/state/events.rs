//! 错误通知与异步保存结果追踪

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
    pub last_error: Option<String>,
}
impl Default for ErrorNotifier {
    fn default() -> Self {
        Self::new()
    }
}
impl Global for ErrorNotifier {}
impl ErrorNotifier {
    pub fn new() -> Self {
        Self { last_error: None }
    }

    pub fn set_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.last_error.take()
    }
}

/// 异步保存结果追踪器
///
/// 用于记录异步保存操作的结果，让主线程能够在适当时机检查并处理。
/// 解决了异步任务无法直接调用 cx.emit() 的问题。
#[derive(Debug, Default)]
pub struct SaveResults {
    /// 成功保存的 item ID 列表
    pub succeeded: Vec<String>,
    /// 保存失败的 item ID 列表
    pub failed: Vec<String>,
}

impl Global for SaveResults {}
impl SaveResults {
    pub fn new() -> Self {
        Self { succeeded: Vec::new(), failed: Vec::new() }
    }

    /// 记录保存成功
    pub fn mark_succeeded(&mut self, item_id: String) {
        self.succeeded.push(item_id);
    }

    /// 记录保存失败
    pub fn mark_failed(&mut self, item_id: String) {
        self.failed.push(item_id);
    }

    /// 检查并取出指定 item 的保存结果
    ///
    /// 返回 `Some(true)` 表示成功，`Some(false)` 表示失败，`None` 表示无结果
    pub fn take_result(&mut self, item_id: &str) -> Option<bool> {
        if let Some(pos) = self.succeeded.iter().position(|id| id == item_id) {
            self.succeeded.remove(pos);
            return Some(true);
        }
        if let Some(pos) = self.failed.iter().position(|id| id == item_id) {
            self.failed.remove(pos);
            return Some(false);
        }
        None
    }

    /// 清空所有结果
    pub fn clear(&mut self) {
        self.succeeded.clear();
        self.failed.clear();
    }
}
