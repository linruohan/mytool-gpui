#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use std::sync::mpsc::Sender;

/// 🚀 7.0新增：应用退出信号发送器
///
/// 由 main.rs 初始化，用于从 GPUI 内部通知主线程执行退出操作。
pub static mut SHUTDOWN_SENDER: Option<Sender<bool>> = None;

use gpui::{
    Action, AnyView, App, AppContext, Bounds, Entity, Focusable, Global, IntoElement, KeyBinding,
    Pixels, SharedString, Size, Styled, Window, WindowBounds, WindowKind, WindowOptions, actions,
    px, size,
};
use gpui_component::{
    Root, TitleBar, WindowExt,
    dock::{PanelInfo, register_panel},
    h_flex,
    scroll::ScrollbarShow,
    text::markdown,
};
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
// 核心模块
pub mod core;

// UI 模块
mod ui;

// 其他模块
mod plugins; // 插件系统
mod utils;

// 重新导出核心模块
pub use core::{
    actions as todo_actions, error_handler::*, services as state_service, shortcuts::*,
    state as todo_state,
};

// 重新导出插件
pub use plugins::*;
// 重新导出 UI 模块
pub use ui::app_menus;
// 内部使用
use ui::layout::{
    story_container::StoryContainer, story_root::StoryRoot, story_section::StorySection,
    story_state::StoryState, title_bar::AppTitleBar,
};
pub use ui::{
    components::*,
    gallery::Gallery,
    stories::*,
    theme::{themes, visual_enhancements::*},
    views::*,
    widgets::*,
};
// 重新导出工具
pub use utils::play_ogg_file;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = mytool, no_json)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = mytool, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = mytool, no_json)]
pub struct SelectFont(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = mytool, no_json)]
pub struct SelectRadius(usize);

actions!(mytool, [
    About,
    Open,
    Quit,
    ToggleSearch,
    TestAction,
    Tab,
    TabPrev,
    ShowPanelInfo,
    ToggleListActiveHighlight
]);
const PANEL_NAME: &str = "StoryContainer";

pub struct AppState {
    pub invisible_panels: Entity<Vec<SharedString>>,
}
impl AppState {
    fn init(cx: &mut App) {
        rust_i18n::set_locale("zh-CN");
        let state = Self { invisible_panels: cx.new(|_| Vec::new()) };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

pub fn create_new_window<F, E>(title: &str, crate_view_fn: F, cx: &mut App)
where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    create_new_window_with_size(title, None, crate_view_fn, cx);
}

pub fn create_new_window_with_size<F, E>(
    title: &str,
    window_size: Option<Size<Pixels>>,
    crate_view_fn: F,
    cx: &mut App,
) where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let mut window_size = window_size.unwrap_or(size(px(1600.0), px(1200.0)));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }
    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size { width: px(480.), height: px(320.) }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let story_root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

                // Set focus to the StoryRoot to enable it's actions.
                let focus_handle = story_root.focus_handle(cx);
                window.defer(cx, move |window, cx| {
                    focus_handle.focus(window, cx);
                });

                cx.new(|cx| {
                    let root = Root::new(story_root, window, cx);
                    root
                })
            })
            .expect("failed to open window");

        window
            .update(cx, |_, window, _| {
                window.activate_window();
                window.set_window_title(&title);
            })
            .expect("failed to update window");

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

impl Global for AppState {}

pub fn init(cx: &mut App) {
    // 初始化日志系统 - 默认显示 info 级别日志
    // 可以通过设置环境变量 RUST_LOG=debug 或 RUST_LOG=mytool=trace 来调整级别
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("info")
            .add_directive("gpui_component=info".parse().unwrap())
            .add_directive("mytool=info".parse().unwrap())
            .add_directive("todos=info".parse().unwrap())
            .add_directive("sqlx=warn".parse().unwrap())
            .add_directive("todos::app::patch=warn".parse().unwrap())
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(env_filter)
        .init();

    gpui_component::init(cx);
    AppState::init(cx);
    themes::init(cx);
    ui::stories::init(cx);

    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", Open, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", Open, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| {
        // 先等待 DB Runtime 中的异步任务完成
        let _ = crate::core::tokio_runtime::shutdown_db_runtime(Some(
            std::time::Duration::from_secs(10),
        ));

        if let Some(db_state) = cx.try_global::<crate::todo_state::DBState>() {
            db_state.shutdown();
        }

        cx.quit();
        request_shutdown();
    });

    cx.on_action(|_: &About, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |root, window, cx| {
                        root.push_notification(
                            "GPUI Component Storybook\nVersion 0.1.0",
                            window,
                            cx,
                        );
                    })
                    .map_err(|e| tracing::error!("failed to push notification: {:?}", e))
                    .ok();
            });
        }
    });

    // Central handlers for panel info and toggle search actions. Panels emit these actions;
    // Handle them globally by finding the active Root window and pushing a notification.
    cx.on_action(|_: &ShowPanelInfo, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |root, window, cx| {
                        root.push_notification("You have clicked panel info.", window, cx);
                    })
                    .map_err(|e| tracing::error!("failed to push notification: {:?}", e))
                    .ok();
            });
        }
    });

    cx.on_action(|_: &About, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.title("About").description(markdown(
                                    "GPUI Component mytool\n\n\
                                    Version 0.1.0\n\n\
                                    https://github/linruohan/gpui-component",
                                ))
                            });
                        });
                    })
                    .unwrap();
            });
        }
    });

    register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            _ => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            },
        };

        let view = cx.new(|cx| {
            let (title, description, closable, zoomable, story, on_active) =
                story_state.to_story(window, cx);
            let mut container = StoryContainer::new(window, cx)
                .story(story, story_state.story_klass)
                .on_active(on_active);

            cx.on_focus_in(&container.focus_handle, window, |this: &mut StoryContainer, _, _| {
                tracing::debug!("StoryContainer focus in: {}", this.name);
            })
            .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        Box::new(view)
    });

    cx.activate(true);
}
pub(crate) fn section(title: impl Into<SharedString>) -> StorySection {
    StorySection {
        title: title.into(),
        sub_title: vec![],
        base: h_flex().w_full().flex_wrap().justify_center().items_center().gap_2(),
        children: vec![],
    }
}

pub(crate) fn section_with_title(title: impl IntoElement) -> StorySection {
    StorySection {
        title: "".into(),
        sub_title: vec![title.into_any_element()],
        base: h_flex().w_full().flex_wrap().justify_center().items_center().gap_2(),
        children: vec![],
    }
}

/// 🚀 7.0新增：请求应用退出
///
/// 发送退出信号到 main.rs 中的后台监控线程，
/// 触发 process::exit(0) 确保进程退出。
/// 用于 Windows 上 X 按钮关闭窗口时的退出路径。
/// ⚠️ 注意：此函数可能在异步上下文（GPUI Drop）中被调用。
///
/// 🚀 优化 (2026-05-17)：移除15秒等待时间，立即发送退出信号
/// 理由：
/// - 连接池会在进程退出时自动清理，无需手动等待
/// - 减少用户等待时间，提升体验
/// - DB 操作会被取消，操作系统会自动关闭文件句柄
pub fn request_shutdown() {
    let sender: Option<std::sync::mpsc::Sender<bool>> =
        unsafe { std::ptr::read(&raw const SHUTDOWN_SENDER) };

    // 直接发送退出信号，不等待15秒
    if let Some(sender) = sender {
        let _ = sender.send(true);
    }
}
