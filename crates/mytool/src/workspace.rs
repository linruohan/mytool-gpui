use anyhow::{Context as _, Result};
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockAreaState, DockEvent, DockItem, DockPlacement},
    popup_menu::PopupMenuExt,
    IconName, Root, Sizable, Theme, TitleBar,
};
use mytool::{AppState, AppTitleBar};
use serde::Deserialize;
const MAIN_DOCK_AREA: DockAreaTab = DockAreaTab {
    id: "main-dock",
    version: 5,
};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct AddPanel(DockPlacement);

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct TogglePanelVisible(SharedString);
struct DockAreaTab {
    id: &'static str,
    version: usize,
}
impl_internal_actions!(mytool, [AddPanel, TogglePanelVisible]);
actions!(mytool, [ToggleDockToggleButton]);
pub fn open_new(
    cx: &mut App,
    init: impl FnOnce(&mut Root, &mut Window, &mut Context<Root>) + 'static + Send,
) -> Task<()> {
    let task: Task<std::result::Result<WindowHandle<Root>, anyhow::Error>> =
        Workspace::new_local(cx);
    cx.spawn(async move |cx| {
        if let Some(root) = task.await.ok() {
            root.update(cx, |workspace, window, cx| init(workspace, window, cx))
                .expect("failed to init workspace");
        }
    })
}
pub struct Workspace {
    title_bar: Entity<AppTitleBar>,
    pub text: SharedString,
}
impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title_bar = cx.new(|cx| {
            AppTitleBar::new("MyTool", window, cx).child({
                move |_, cx| {
                    Button::new("add-panel")
                        .icon(IconName::LayoutDashboard)
                        .small()
                        .ghost()
                        .popup_menu({
                            let invisible_panels = AppState::global(cx).invisible_panels.clone();

                            move |menu, _, cx| {
                                menu.menu(
                                    "Add Panel to Center",
                                    Box::new(AddPanel(DockPlacement::Center)),
                                )
                                .separator()
                                .menu("Add Panel to Left", Box::new(AddPanel(DockPlacement::Left)))
                                .menu(
                                    "Add Panel to Right",
                                    Box::new(AddPanel(DockPlacement::Right)),
                                )
                                .menu(
                                    "Add Panel to Bottom",
                                    Box::new(AddPanel(DockPlacement::Bottom)),
                                )
                                .separator()
                                .menu(
                                    "Show / Hide Dock Toggle Button",
                                    Box::new(ToggleDockToggleButton),
                                )
                                .separator()
                                .menu_with_check(
                                    "Sidebar",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Sidebar")),
                                    Box::new(TogglePanelVisible(SharedString::from("Sidebar"))),
                                )
                                .menu_with_check(
                                    "Modal",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Modal")),
                                    Box::new(TogglePanelVisible(SharedString::from("Modal"))),
                                )
                                .menu_with_check(
                                    "Accordion",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Accordion")),
                                    Box::new(TogglePanelVisible(SharedString::from("Accordion"))),
                                )
                                .menu_with_check(
                                    "List",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("List")),
                                    Box::new(TogglePanelVisible(SharedString::from("List"))),
                                )
                            }
                        })
                        .anchor(Corner::TopRight)
                }
            })
        });
        Self {
            title_bar,
            text: "World".into(),
        }
    }
    pub fn new_local(cx: &mut App) -> Task<anyhow::Result<WindowHandle<Root>>> {
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                #[cfg(not(target_os = "linux"))]
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(640.),
                    height: px(480.),
                }),
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            let window = cx.open_window(options, |window, cx| {
                let story_view = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| Root::new(story_view.into(), window, cx))
            })?;

            window
                .update(cx, |_, window, cx| {
                    window.activate_window();
                    window.set_window_title("GPUI App");
                    cx.on_release(|_, cx| {
                        // exit app
                        cx.quit();
                    })
                    .detach();
                })
                .expect("failed to update window");

            Ok(window)
        })
    }
}
impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer_layer = Root::render_drawer_layer(window, cx);
        let modal_layer = Root::render_modal_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .id("story-workspace")
            // .on_action(cx.listener(Self::on_action_add_panel))
            // .on_action(cx.listener(Self::on_action_toggle_panel_visible))
            // .on_action(cx.listener(Self::on_action_toggle_dock_toggle_button))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            // .child(self.dock_area.clone())
            .children(drawer_layer)
            .children(modal_layer)
            .child(div().absolute().top_8().children(notification_layer))
    }
}
