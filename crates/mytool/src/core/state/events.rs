//! 错误通知与异步保存结果追踪
//!
//! 邮箱字段用 Mutex，观察者消费时不必 `update_global`，避免再次通知自己卡死。

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

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

const ERROR_REPEAT_WINDOW: std::time::Duration = std::time::Duration::from_secs(4);

/// 错误通知器
///
/// 后台任务把错误排队，UI 观察后逐条弹出。不同的错误都会保留。
/// 同一句错误在几秒内重复出现时只保留一条，避免失败重试刷屏。
pub struct ErrorNotifier {
    pending: Mutex<Vec<String>>,
    recent: Mutex<Option<(String, std::time::Instant)>>,
}
impl Default for ErrorNotifier {
    fn default() -> Self {
        Self::new()
    }
}
impl Global for ErrorNotifier {}
impl ErrorNotifier {
    pub fn new() -> Self {
        Self { pending: Mutex::new(Vec::new()), recent: Mutex::new(None) }
    }

    pub fn set_error(&self, message: String) {
        let mut recent = self.recent.lock().unwrap();
        if let Some((last, at)) = recent.as_ref()
            && last == &message
            && at.elapsed() < ERROR_REPEAT_WINDOW
        {
            return;
        }
        *recent = Some((message.clone(), std::time::Instant::now()));
        drop(recent);
        self.pending.lock().unwrap().push(message);
    }

    pub fn take_error(&self) -> Option<String> {
        let mut pending = self.pending.lock().unwrap();
        if pending.is_empty() { None } else { Some(pending.remove(0)) }
    }

    pub fn take_all(&self) -> Vec<String> {
        std::mem::take(&mut *self.pending.lock().unwrap())
    }

    pub fn peek_error(&self) -> bool {
        !self.pending.lock().unwrap().is_empty()
    }
}

/// 到期提醒通知器（后台轮询写入，UI 观察后弹出）
#[derive(Clone, Debug)]
pub struct ReminderNotice {
    pub reminder_id: String,
    pub item_id: String,
    pub title: String,
}

pub struct ReminderNotifier {
    pending: Mutex<Vec<ReminderNotice>>,
    /// 已交给界面、数据库尚未改完（软删除或延后）的提醒。
    held: Mutex<HashSet<String>>,
    /// 用户点了延后，关闭回调不要再把它标成已触发。
    snoozed: Mutex<HashSet<String>>,
}
impl Default for ReminderNotifier {
    fn default() -> Self {
        Self::new()
    }
}
impl Global for ReminderNotifier {}
impl ReminderNotifier {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            held: Mutex::new(HashSet::new()),
            snoozed: Mutex::new(HashSet::new()),
        }
    }

    /// 同一条提醒在排队或已弹出时不再重复入队。返回是否新写入。
    pub fn push(&self, notice: ReminderNotice) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let held = self.held.lock().unwrap();
        if held.contains(&notice.reminder_id)
            || pending.iter().any(|item| item.reminder_id == notice.reminder_id)
        {
            return false;
        }
        drop(held);
        pending.push(notice);
        true
    }

    pub fn take_all(&self) -> Vec<ReminderNotice> {
        std::mem::take(&mut *self.pending.lock().unwrap())
    }

    pub fn hold(&self, id: &str) {
        self.held.lock().unwrap().insert(id.to_string());
    }

    pub fn release(&self, id: &str) {
        self.held.lock().unwrap().remove(id);
    }

    pub fn begin_snooze(&self, id: &str) {
        self.snoozed.lock().unwrap().insert(id.to_string());
    }

    pub fn take_snooze(&self, id: &str) -> bool {
        self.snoozed.lock().unwrap().remove(id)
    }
}

/// 提醒被关掉或改期后，通知已经打开的提醒按钮刷新。
#[derive(Clone, Debug)]
pub enum ReminderChange {
    Dismissed { reminder_id: String, item_id: String },
    Rescheduled { reminder_id: String, item_id: String, due: String },
}

pub struct ReminderChanges {
    seq: u64,
    latest: Vec<ReminderChange>,
    dismissed: HashSet<String>,
    rescheduled: HashMap<String, String>,
}

impl Default for ReminderChanges {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for ReminderChanges {}

impl ReminderChanges {
    pub fn new() -> Self {
        Self { seq: 0, latest: Vec::new(), dismissed: HashSet::new(), rescheduled: HashMap::new() }
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn push(&mut self, change: ReminderChange) {
        self.seq = self.seq.wrapping_add(1);
        match &change {
            ReminderChange::Dismissed { reminder_id, .. } => {
                self.dismissed.insert(reminder_id.clone());
                self.rescheduled.remove(reminder_id);
            },
            ReminderChange::Rescheduled { reminder_id, due, .. } => {
                self.dismissed.remove(reminder_id);
                self.rescheduled.insert(reminder_id.clone(), due.clone());
            },
        }
        self.latest = vec![change];
    }

    pub fn is_dismissed(&self, reminder_id: &str) -> bool {
        self.dismissed.contains(reminder_id)
    }

    pub fn rescheduled_due(&self, reminder_id: &str) -> Option<String> {
        self.rescheduled.get(reminder_id).cloned()
    }

    pub fn latest(&self) -> (u64, Vec<ReminderChange>) {
        (self.seq, self.latest.clone())
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

const NAV_HISTORY_LIMIT: usize = 50;

/// 侧栏看板或某个项目。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavPlace {
    Board(usize),
    Project(String),
}

/// 看板/项目访问历史，供后退前进使用。
#[derive(Clone, Debug, Default)]
pub struct NavHistory {
    back: Vec<NavPlace>,
    forward: Vec<NavPlace>,
}

impl NavHistory {
    pub fn record_leaving(&mut self, leaving: NavPlace, arriving: &NavPlace) {
        if &leaving == arriving {
            return;
        }
        self.back.push(leaving);
        if self.back.len() > NAV_HISTORY_LIMIT {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    pub fn go_back(&mut self, current: NavPlace) -> Option<NavPlace> {
        let dest = self.back.pop()?;
        self.forward.push(current);
        Some(dest)
    }

    pub fn go_forward(&mut self, current: NavPlace) -> Option<NavPlace> {
        let dest = self.forward.pop()?;
        self.back.push(current);
        Some(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorNotifier, NavHistory, NavPlace, ReminderChange, ReminderChanges, ReminderNotice,
        ReminderNotifier, step_visible_id,
    };

    #[test]
    fn error_notifier_keeps_every_message() {
        let notifier = ErrorNotifier::new();
        notifier.set_error("a".into());
        notifier.set_error("b".into());
        assert_eq!(notifier.take_all(), vec!["a".to_string(), "b".to_string()]);
        assert!(notifier.take_all().is_empty());
        assert!(!notifier.peek_error());
    }

    #[test]
    fn error_notifier_drops_immediate_repeat() {
        let notifier = ErrorNotifier::new();
        notifier.set_error("a".into());
        notifier.set_error("a".into());
        notifier.set_error("b".into());
        assert_eq!(notifier.take_all(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn reminder_notifier_dedupes_while_queued_or_held() {
        let notifier = ReminderNotifier::new();
        let notice = ReminderNotice {
            reminder_id: "1".into(),
            item_id: "item".into(),
            title: "task".into(),
        };
        assert!(notifier.push(notice.clone()));
        assert!(!notifier.push(notice.clone()));
        assert_eq!(notifier.take_all().len(), 1);
        notifier.hold("1");
        assert!(!notifier.push(notice.clone()));
        notifier.begin_snooze("1");
        assert!(notifier.take_snooze("1"));
        assert!(!notifier.take_snooze("1"));
        notifier.release("1");
        assert!(notifier.push(notice));
    }

    #[test]
    fn reminder_changes_replace_the_latest_event() {
        let mut changes = ReminderChanges::new();
        assert_eq!(changes.seq(), 0);
        changes.push(ReminderChange::Dismissed { reminder_id: "r".into(), item_id: "i".into() });
        changes.push(ReminderChange::Rescheduled {
            reminder_id: "r".into(),
            item_id: "i".into(),
            due: "2026-10-24 10:15:00".into(),
        });
        let (seq, latest) = changes.latest();
        assert_eq!(seq, 2);
        assert!(matches!(latest.as_slice(), [ReminderChange::Rescheduled { .. }]));
        assert!(!changes.is_dismissed("r"));
        assert_eq!(changes.rescheduled_due("r").as_deref(), Some("2026-10-24 10:15:00"));

        changes
            .push(ReminderChange::Dismissed { reminder_id: "other".into(), item_id: "i".into() });
        assert!(changes.is_dismissed("other"));
        assert_eq!(changes.rescheduled_due("r").as_deref(), Some("2026-10-24 10:15:00"));
    }

    #[test]
    fn step_visible_id_wraps_and_defaults() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(step_visible_id(&ids, None, 1).as_deref(), Some("a"));
        assert_eq!(step_visible_id(&ids, None, -1).as_deref(), Some("c"));
        assert_eq!(step_visible_id(&ids, Some("a"), 1).as_deref(), Some("b"));
        assert_eq!(step_visible_id(&ids, Some("c"), 1).as_deref(), Some("a"));
        assert_eq!(step_visible_id(&ids, Some("a"), -1).as_deref(), Some("c"));
    }

    #[test]
    fn nav_history_back_forward_and_branch_clears_forward() {
        let mut nav = NavHistory::default();
        nav.record_leaving(NavPlace::Board(0), &NavPlace::Project("p1".into()));
        nav.record_leaving(NavPlace::Project("p1".into()), &NavPlace::Board(1));
        assert_eq!(nav.go_back(NavPlace::Board(1)), Some(NavPlace::Project("p1".into())));
        assert_eq!(nav.go_forward(NavPlace::Project("p1".into())), Some(NavPlace::Board(1)));
        nav.record_leaving(NavPlace::Board(1), &NavPlace::Board(2));
        assert_eq!(nav.go_forward(NavPlace::Board(2)), None);
        assert_eq!(nav.go_back(NavPlace::Board(2)), Some(NavPlace::Board(1)));
    }

    #[test]
    fn nav_history_skips_same_place() {
        let mut nav = NavHistory::default();
        nav.record_leaving(NavPlace::Board(0), &NavPlace::Board(0));
        assert_eq!(nav.go_back(NavPlace::Board(0)), None);
    }
}
