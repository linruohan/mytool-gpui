//! 错误通知与异步保存结果追踪
//!
//! 邮箱字段用 Mutex，观察者消费时不必 `update_global`，避免再次通知自己卡死。

use std::{collections::HashSet, sync::Mutex};

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

/// 看板多选（Ctrl/Cmd+点击）。独立 Global，避免误触发 TodoStore 全量刷新。
#[derive(Default)]
pub struct ItemSelection {
    ids: HashSet<String>,
    primary: Option<String>,
}

impl Global for ItemSelection {}

impl ItemSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ids(&self) -> &HashSet<String> {
        &self.ids
    }

    pub fn primary_id(&self) -> Option<&str> {
        self.primary.as_deref().or_else(|| self.ids.iter().next().map(String::as_str))
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.primary = None;
    }

    pub fn set_ids(&mut self, ids: HashSet<String>) {
        self.primary = ids.iter().next().cloned();
        self.ids = ids;
    }

    pub fn select_only(&mut self, id: String) {
        self.apply_click(id, false);
    }

    /// 普通点击：只保留这一项。修饰键点击：切换该项。
    pub fn apply_click(&mut self, id: String, multi: bool) {
        if multi {
            if !self.ids.remove(&id) {
                self.ids.insert(id.clone());
                self.primary = Some(id);
            } else if self.primary.as_deref() == Some(id.as_str()) {
                self.primary = self.ids.iter().next().cloned();
            }
        } else {
            self.ids.clear();
            self.ids.insert(id.clone());
            self.primary = Some(id);
        }
    }
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

/// 到期提醒通知器（后台轮询写入，UI 观察后弹出）
pub struct ReminderNotifier {
    pending: Mutex<Vec<String>>,
}
impl Default for ReminderNotifier {
    fn default() -> Self {
        Self::new()
    }
}
impl Global for ReminderNotifier {}
impl ReminderNotifier {
    pub fn new() -> Self {
        Self { pending: Mutex::new(Vec::new()) }
    }

    pub fn push(&self, message: String) {
        self.pending.lock().unwrap().push(message);
    }

    pub fn take_all(&self) -> Vec<String> {
        std::mem::take(&mut *self.pending.lock().unwrap())
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

/// 在可见列表里按 delta 移动当前项（循环）。无选中时：向下取第一项，向上取最后一项。
pub fn step_visible_id(ids: &[String], current: Option<&str>, delta: i32) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let n = ids.len() as i32;
    let ix = match current.and_then(|id| ids.iter().position(|x| x == id)) {
        Some(i) => (i as i32 + delta).rem_euclid(n) as usize,
        None if delta < 0 => ids.len() - 1,
        None => 0,
    };
    Some(ids[ix].clone())
}

#[cfg(test)]
mod tests {
    use super::step_visible_id;

    #[test]
    fn step_visible_id_wraps_and_defaults() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(step_visible_id(&ids, None, 1).as_deref(), Some("a"));
        assert_eq!(step_visible_id(&ids, None, -1).as_deref(), Some("c"));
        assert_eq!(step_visible_id(&ids, Some("a"), 1).as_deref(), Some("b"));
        assert_eq!(step_visible_id(&ids, Some("c"), 1).as_deref(), Some("a"));
        assert_eq!(step_visible_id(&ids, Some("a"), -1).as_deref(), Some("c"));
    }
}
