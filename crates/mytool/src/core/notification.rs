/// 统一的通知和日志系统
/// 结合 tracing 日志和 GPUI 通知，提供一致的用户反馈
use gpui::{App, Window};
use gpui_component::WindowExt;

/// 通知系统
pub struct NotificationSystem;

impl NotificationSystem {
    /// 显示成功通知
    pub fn success(message: impl Into<String>, window: &mut Window, cx: &mut App) {
        let msg = message.into();
        tracing::info!("✓ {}", msg);
        window.push_notification(format!("✓ {}", msg), cx);
    }

    /// 显示错误通知
    pub fn error(message: impl Into<String>, window: &mut Window, cx: &mut App) {
        let msg = message.into();
        tracing::error!("✗ {}", msg);
        window.push_notification(format!("✗ {}", msg), cx);
    }

    /// 记录调试信息（仅日志，不显示通知）
    pub fn debug(message: impl AsRef<str>) {
        tracing::debug!("{}", message.as_ref());
    }

    /// 记录错误但不显示通知（用于后台操作）
    pub fn log_error(message: impl AsRef<str>, error: impl std::fmt::Debug) {
        tracing::error!("{}: {:?}", message.as_ref(), error);
    }
}

/// 扩展 trait，为 Window 添加便捷的通知方法
pub trait NotificationExt {
    fn notify_success(&mut self, message: impl Into<String>, cx: &mut App);
    fn notify_error(&mut self, message: impl Into<String>, cx: &mut App);
}

impl NotificationExt for Window {
    fn notify_success(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::success(message, self, cx);
    }

    fn notify_error(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::error(message, self, cx);
    }
}
