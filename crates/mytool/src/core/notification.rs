/// 统一的通知和日志系统
/// 结合 tracing 日志和 GPUI 通知，提供一致的用户反馈
use gpui::{App, Window};
use gpui_component::WindowExt;

/// 通知级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// 信息提示
    Info,
    /// 成功操作
    Success,
    /// 警告信息
    Warning,
    /// 错误信息
    Error,
}

/// 通知系统
pub struct NotificationSystem;

impl NotificationSystem {
    /// 显示信息通知
    ///
    /// 同时记录 info 级别日志并显示 UI 通知
    pub fn info(message: impl Into<String>, window: &mut Window, cx: &mut App) {
        let msg = message.into();
        tracing::info!("{}", msg);
        window.push_notification(msg, cx);
    }

    /// 显示成功通知
    ///
    /// 同时记录 info 级别日志并显示 UI 通知
    pub fn success(message: impl Into<String>, window: &mut Window, cx: &mut App) {
        let msg = message.into();
        tracing::info!("✓ {}", msg);
        window.push_notification(format!("✓ {}", msg), cx);
    }

    /// 显示警告通知
    ///
    /// 同时记录 warn 级别日志并显示 UI 通知
    pub fn warning(message: impl Into<String>, window: &mut Window, cx: &mut App) {
        let msg = message.into();
        tracing::warn!("⚠ {}", msg);
        window.push_notification(format!("⚠ {}", msg), cx);
    }

    /// 显示错误通知
    ///
    /// 同时记录 error 级别日志并显示 UI 通知
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

    /// 记录警告但不显示通知（用于后台操作）
    pub fn log_warning(message: impl AsRef<str>) {
        tracing::warn!("{}", message.as_ref());
    }
}

/// 扩展 trait，为 Window 添加便捷的通知方法
pub trait NotificationExt {
    fn notify_info(&mut self, message: impl Into<String>, cx: &mut App);
    fn notify_success(&mut self, message: impl Into<String>, cx: &mut App);
    fn notify_warning(&mut self, message: impl Into<String>, cx: &mut App);
    fn notify_error(&mut self, message: impl Into<String>, cx: &mut App);
}

impl NotificationExt for Window {
    fn notify_info(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::info(message, self, cx);
    }

    fn notify_success(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::success(message, self, cx);
    }

    fn notify_warning(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::warning(message, self, cx);
    }

    fn notify_error(&mut self, message: impl Into<String>, cx: &mut App) {
        NotificationSystem::error(message, self, cx);
    }
}

/// 异步操作结果通知
///
/// 用于异步操作完成后的通知
pub trait AsyncNotify {
    /// 成功时通知
    fn notify_success(self, message: impl Into<String>) -> Self;

    /// 失败时通知
    fn notify_error(self, message: impl Into<String>) -> Self;
}

impl<T, E: std::fmt::Debug> AsyncNotify for Result<T, E> {
    fn notify_success(self, message: impl Into<String>) -> Self {
        if self.is_ok() {
            tracing::info!("✓ {}", message.into());
        }
        self
    }

    fn notify_error(self, message: impl Into<String>) -> Self {
        if let Err(ref e) = self {
            tracing::error!("✗ {}: {:?}", message.into(), e);
        }
        self
    }
}

/// 便捷宏：在有 window 和 cx 的上下文中使用
#[macro_export]
macro_rules! notify_info {
    ($window:expr, $cx:expr, $($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::info(
            format!($($arg)*),
            $window,
            $cx
        )
    };
}

#[macro_export]
macro_rules! notify_success {
    ($window:expr, $cx:expr, $($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::success(
            format!($($arg)*),
            $window,
            $cx
        )
    };
}

#[macro_export]
macro_rules! notify_warning {
    ($window:expr, $cx:expr, $($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::warning(
            format!($($arg)*),
            $window,
            $cx
        )
    };
}

#[macro_export]
macro_rules! notify_error {
    ($window:expr, $cx:expr, $($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::error(
            format!($($arg)*),
            $window,
            $cx
        )
    };
}

/// 便捷宏：仅记录日志，不显示通知
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::debug(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($msg:expr, $err:expr) => {
        $crate::core::notification::NotificationSystem::log_error($msg, $err)
    };
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        $crate::core::notification::NotificationSystem::log_warning(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_notify() {
        let result: Result<(), &str> = Ok(());
        let _ = result.notify_success("Operation completed");

        let result: Result<(), &str> = Err("test error");
        let _ = result.notify_error("Operation failed");
    }
}
