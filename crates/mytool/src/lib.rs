#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use crate::core::state::PendingTasksState;
use gpui::{
    Action, AnyView, App, AppContext, Bounds, Entity, Focusable, Global, KeyBinding, Pixels,
    SharedString, Size, Styled, Window, WindowBounds, WindowKind, WindowOptions, actions, px, size,
};
use gpui_component::{
    Root, TitleBar, WindowExt,
    dock::{PanelInfo, register_panel},
    h_flex,
    scroll::ScrollbarShow,
};
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
// 核心模块
mod core;

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
    component_manager::ComponentManager,
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

actions!(
    mytool,
    [
        About,
        Open,
        Quit,
        CloseWindow,
        ToggleSearch,
        TestAction,
        Tab,
        TabPrev,
        ShowPanelInfo,
        ToggleListActiveHighlight
    ]
);
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

        let window_result = cx.open_window(options, |window, cx| {
            let view = crate_view_fn(window, cx);
            let story_root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

            // Set focus to the StoryRoot to enable it's actions.
            let focus_handle = story_root.focus_handle(cx);
            window.defer(cx, move |window, cx| {
                focus_handle.focus(window, cx);
            });

            cx.new(|cx| Root::new(story_root, window, cx))
        });

        let window = match window_result {
            Ok(win) => win,
            Err(e) => {
                tracing::error!("failed to open window: {:?}", e);
                return Ok::<_, anyhow::Error>(());
            },
        };

        if let Err(e) = window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        }) {
            tracing::error!("failed to update window: {:?}", e);
        }

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
        // 🚀 检查是否有未完成的保存任务
        let pending_count = cx.global::<PendingTasksState>().pending_count();

        if pending_count > 0 {
            tracing::info!(
                "🔄 Quit requested but {} pending tasks, waiting for completion...",
                pending_count
            );

            // 异步等待任务完成后再退出
            cx.spawn(async move |cx| {
                // 等待所有任务完成（最多等待 5 秒）
                let max_wait = std::time::Duration::from_secs(5);
                let start = std::time::Instant::now();
                let check_interval = std::time::Duration::from_millis(100);

                loop {
                    let remaining =
                        cx.update_global::<PendingTasksState, _>(|state, _| state.pending_count());

                    if remaining == 0 {
                        tracing::info!("✅ All pending tasks completed, quitting...");
                        break;
                    }

                    if start.elapsed() >= max_wait {
                        tracing::warn!(
                            "⚠️ Timeout waiting for {} pending tasks, forcing quit...",
                            remaining
                        );
                        break;
                    }

                    // 等待一小段时间再检查
                    tokio::time::sleep(check_interval).await;
                }

                // 退出应用
                cx.update(|cx| {
                    cx.quit();
                });
            })
            .detach();
        } else {
            tracing::info!("✅ No pending tasks, quitting immediately...");
            cx.quit();
        }
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

    cx.on_action(|_: &ToggleSearch, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_root, window, cx| {
                        // Respect focused input: if an input is focused, ignore toggle
                        if window.has_focused_input(cx) {
                            return;
                        }
                        window.push_notification("You have toggled search.", cx);
                    })
                    .map_err(|e| tracing::error!("failed to push notification: {:?}", e))
                    .ok();
            });
        }
    });

    register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => {
                serde_json::from_value::<StoryState>(value.clone()).unwrap_or_else(|e| {
                    tracing::error!(
                        "failed to deserialize panel StoryState: {:?}. Falling back to ListStory",
                        e
                    );
                    // Fallback to a default StoryState that points to ListStory
                    StoryState { story_klass: SharedString::from("ListStory") }
                })
            },
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
                println!("StoryContainer focus in: {}", this.name);
            })
            .detach();

            container.name = SharedString::from(title);
            container.description = SharedString::from(description);
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
        base: h_flex().flex_wrap().justify_center().items_center().w_full().gap_4(),
        children: vec![],
    }
}
