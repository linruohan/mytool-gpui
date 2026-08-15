//! lib.rs - gpui-fps 入口（关键修改：函数名适配新版 gpui）
//!
//! 主要改动点：
//! 1. set_frame_trace_enabled() → set_trace_enabled()
//! 2. frame_trace_enabled() → trace_enabled()

mod monitor;
mod overlay;
mod sampler;
mod style;

use std::{collections::HashMap, sync::Mutex};

use gpui::{App, AppContext as _, Entity, Global, Window, WindowId};
pub use monitor::FpsMonitor;
pub use overlay::FpsOverlay;

/// 创建并显示 FPS HUD（固定在父容器右上角）。
///
/// 父元素必须设置为 `relative()`。同一窗口最多调用一次。
/// 当 HUD 显示时会自动启用 GPUI 的全局 frame tracing，隐藏时自动恢复。
pub fn fps_monitor(window: &mut Window, cx: &mut App) -> FpsOverlay {
    let window_id = window.window_handle().window_id();
    // 尝试复用已有的 monitor（按窗口 ID 存储为全局）
    let existing = cx.try_global::<Monitors>().and_then(|state| state.0.get(&window_id).cloned());
    let monitor = match existing {
        Some(monitor) => monitor,
        None => {
            let monitor = cx.new(|cx| FpsMonitor::new(window, cx));
            cx.default_global::<Monitors>().0.insert(window_id, monitor.clone());
            monitor
        },
    };

    FpsOverlay::new(&monitor)
}

/// 每个窗口一个 FpsMonitor 的全局映射表
#[derive(Default)]
struct Monitors(HashMap<WindowId, Entity<FpsMonitor>>);

impl Global for Monitors {}

struct TraceState {
    /// 存活的 FrameTraceGuard 引用计数
    refs: usize,
    /// 第一个 guard 被创建时 frame tracing 是否已在外部开启，
    /// 如果已经开启，那我们不负责关闭它
    owned_by_host: bool,
}

static TRACE_STATE: Mutex<TraceState> = Mutex::new(TraceState { refs: 0, owned_by_host: false });

/// 引用计数式的帧追踪开关守卫。
///
/// GPUI 的 set_trace_enabled() 是进程级开关，且关闭会清空环形缓冲区。
/// 因此使用引用计数：最后一个 guard drop 时才真正关闭 tracing，
/// 如果外部已经开启了 tracing，则我们完全不干预开关。
pub(crate) struct FrameTraceGuard {
    _private: (),
}

impl FrameTraceGuard {
    /// 获取追踪守卫（引用计数 +1，如果是第一个则开启追踪）
    pub(crate) fn acquire() -> Self {
        if let Ok(mut state) = TRACE_STATE.lock() {
            if state.refs == 0 {
                // 🔧 修复点：set_frame_trace_enabled → set_trace_enabled
                // 返回 false 时表示原本就是 true，说明 tracing 是外部打开的，我们不拥有它
                state.owned_by_host = !gpui::set_trace_enabled(true);
            }
            state.refs += 1;
        }
        Self { _private: () }
    }
}

impl Drop for FrameTraceGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = TRACE_STATE.lock() {
            state.refs = state.refs.saturating_sub(1);
            // 引用计数归零且 tracing 是由我们开启的，才负责关闭
            if state.refs == 0 && !state.owned_by_host {
                // 🔧 修复点：set_frame_trace_enabled → set_trace_enabled
                gpui::set_trace_enabled(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_guards_keep_tracing_on_until_the_last_one_drops() {
        let outer = FrameTraceGuard::acquire();
        let inner = FrameTraceGuard::acquire();
        // 🔧 修复点：frame_trace_enabled → trace_enabled
        assert!(gpui::trace_enabled());

        drop(inner);
        assert!(gpui::trace_enabled(), "the outer guard still needs the trace");

        drop(outer);
        assert!(!gpui::trace_enabled());
    }
}
