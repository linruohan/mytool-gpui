#![feature(str_as_str)]

mod assets;
mod calendar_story;
mod color_picker_story;
mod date_picker_story;
mod gallery;
mod layouts;
mod table_story;
mod themes;
mod title_bar;
// mod todo_story;
mod todos_view; // 任务管理
mod utils;
mod welcome_story;
mod sidebar_story;

pub use assets::Assets;
pub use gallery::Gallery;
use gpui::{
    actions, div, prelude::FluentBuilder as _, px, rems, size, Action, AnyElement, AnyView, App,
    AppContext, Bounds, Context, Div, Entity, EventEmitter, Focusable, Global, Hsla,
    InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, ParentElement, Render, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowKind,
    WindowOptions,
};
pub use todos_view::{
    Board, BoardType, CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ProjectItem,
    ScheduledBoard, TodayBoard,
};
pub use utils::play_ogg_file;

pub use calendar_story::CalendarStory;
pub use sidebar_story::SidebarStory;

pub use color_picker_story::ColorPickerStory;
pub use date_picker_story::DatePickerStory;
use serde::{Deserialize, Serialize};
pub use table_story::TableStory;
pub use title_bar::AppTitleBar;
// pub use todo_story::TodoStory;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
pub use welcome_story::WelcomeStory;

use gpui_component::{
    button::Button,
    context_menu::ContextMenuExt,
    dock::{register_panel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    h_flex,
    notification::Notification,
    popup_menu::PopupMenu,
    scroll::ScrollbarShow,
    v_flex, ActiveTheme, ContextModal, IconName, Root, TitleBar,
};

rust_i18n::i18n!("locales", fallback = "en");

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

actions!(mytool, [Quit, Open, CloseWindow, ToggleSearch]);

const PANEL_NAME: &str = "StoryContainer";

actions!(mytool, [TestAction, Tab, TabPrev]);

pub struct AppState {
    pub invisible_panels: Entity<Vec<SharedString>>,
    pub theme_name: Option<SharedString>,
}
impl AppState {
    fn init(cx: &mut App) {
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
            theme_name: None,
        };
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
    let mut window_size = size(px(1600.0), px(1200.0));
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
            window_min_size: Some(gpui::Size {
                width: px(640.),
                height: px(480.),
            }),
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
                let root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

                cx.new(|cx| Root::new(root.into(), window, cx))
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

struct StoryRoot {
    title_bar: Entity<AppTitleBar>,
    view: AnyView,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self {
            title_bar,
            view: view.into(),
        }
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer_layer = Root::render_drawer_layer(window, cx);
        let modal_layer = Root::render_modal_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            .children(drawer_layer)
            .children(modal_layer)
            .child(div().absolute().top_8().children(notification_layer))
    }
}

impl Global for AppState {}

pub fn init(cx: &mut App) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gpui_component=trace".parse().unwrap()),
        )
        .init();

    gpui_component::init(cx);
    AppState::init(cx);
    // input_story::init(cx);
    // number_input_story::init(cx);
    // textarea_story::init(cx);
    // dropdown_story::init(cx);
    // popover_story::init(cx);
    // menu_story::init(cx);
    // webview_story::init(cx);
    // tooltip_story::init(cx);
    // otp_input_story::init(cx);

    // let http_client = std::sync::Arc::new(
    //     reqwest_client::ReqwestClient::user_agent("gpui-component/story").unwrap(),
    // );
    // cx.set_http_client(http_client);

    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            _ => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            }
        };

        let view = cx.new(|cx| {
            let (title, description, closable, zoomable, mytool, on_active) =
                story_state.to_story(window, cx);
            let mut container = StoryContainer::new(window, cx)
                .mytool(mytool, story_state.story_klass)
                .on_active(on_active);

            cx.on_focus_in(
                &container.focus_handle,
                window,
                |this: &mut StoryContainer, _, _| {
                    println!("StoryContainer focus in: {}", this.name);
                },
            )
              .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        Box::new(view)
    });

    use gpui_component::input::{Copy, Cut, Paste, Redo, Undo};
    cx.set_menus(vec![
        Menu {
            name: "GPUI App".into(),
            items: vec![MenuItem::action("Quit", Quit)],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![],
        },
    ]);
    cx.activate(true);
}

actions!(mytool, [ShowPanelInfo]);

#[derive(IntoElement)]
struct StorySection {
    base: Div,
    title: AnyElement,
    children: Vec<AnyElement>,
}

impl StorySection {
    #[allow(unused)]
    fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    #[allow(unused)]
    fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    #[allow(unused)]
    fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    #[allow(unused)]
    fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item=AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .mb_5()
            .w_full()
            .child(
                h_flex()
                    .justify_between()
                    .w_full()
                    .gap_4()
                    .child(self.title),
            )
            .child(
                v_flex()
                    .p_4()
                    .overflow_x_hidden()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .items_center()
                    .justify_center()
                    .child(self.base.children(self.children)),
            )
    }
}

impl ContextMenuExt for StorySection {}

pub(crate) fn section(title: impl IntoElement) -> StorySection {
    StorySection {
        title: title.into_any_element(),
        base: h_flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .w_full()
            .gap_4(),
        children: vec![],
    }
}

pub struct StoryContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    mytool: Option<AnyView>,
    story_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
}

#[derive(Debug)]
pub enum ContainerEvent {
    Close,
}

pub trait Mytool: Focusable + Render + Sized {
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    fn title() -> &'static str;
    fn description() -> &'static str {
        ""
    }
    fn closable() -> bool {
        true
    }
    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }
    fn title_bg() -> Option<Hsla> {
        None
    }
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable>;

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }
    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
                     where
                         Self: 'static,
    {
        if let Some(mytool) = view.downcast::<Self>().ok() {
            cx.update_entity(&mytool, |mytool, cx| {
                mytool.on_active(active, window, cx);
            });
        }
    }
}

impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            mytool: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            on_active: None,
        }
    }

    pub fn panel<S: Mytool>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let mytool = S::new_view(window, cx);
        let story_klass = S::klass();
        let focus_handle = mytool.focus_handle(cx);

        let view = cx.new(|cx| {
            let mut mytool = Self::new(window, cx)
                .mytool(mytool.into(), story_klass)
                .on_active(S::on_active_any);
            mytool.focus_handle = focus_handle;
            mytool.closable = S::closable();
            mytool.zoomable = S::zoomable();
            mytool.name = name.into();
            mytool.description = description.into();
            mytool.title_bg = S::title_bg();
            mytool
        });

        view
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn mytool(mut self, mytool: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.mytool = Some(mytool);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }

    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new()
            .message(format!("You have clicked panel info on: {}", self.name))
            .id::<Info>();
        window.push_notification(note, cx);
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message(format!("You have toggled search on: {}", self.name))
            .id::<Search>();
        window.push_notification(note, cx);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString,
}

impl StoryState {
    fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    fn to_story(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        macro_rules! mytool {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        match self.story_klass.to_string().as_str() {
            "CalendarStory" => mytool!(CalendarStory),
            "TableStory" => mytool!(TableStory),
            // "TodoStory" => mytool!(TodoStory),
            "ColorPickerStory" => mytool!(ColorPickerStory),
            "DatePickerStory" => mytool!(DatePickerStory),
            _ => {
                unreachable!("Invalid mytool klass: {}", self.story_klass)
            }
        }
    }
}

impl Panel for StoryContainer {
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        self.name.clone().into_any_element()
    }

    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle {
                background: bg,
                foreground: cx.theme().foreground,
            })
        } else {
            None
        }
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx)
            .invisible_panels
            .read(cx)
            .contains(&self.name)
    }

    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut App) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut App) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active {
            if let Some(mytool) = self.mytool.clone() {
                on_active(mytool, active, _window, cx);
            }
        }
    }

    fn popup_menu(&self, menu: PopupMenu, _window: &Window, _cx: &App) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    fn toolbar_buttons(&self, _window: &mut Window, _cx: &mut App) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info")
                .icon(IconName::Info)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked info button", cx);
                }),
            Button::new("search")
                .icon(IconName::Search)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked search button", cx);
                }),
        ])
    }

    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        let story_state = StoryState {
            story_klass: self.story_klass.clone().unwrap(),
        };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("mytool-container")
            .size_full()
            .overflow_y_scroll()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .when_some(self.mytool.clone(), |this, mytool| {
                this.child(
                    v_flex()
                        .id("mytool-children")
                        .w_full()
                        .flex_1()
                        .p_4()
                        .child(mytool),
                )
            })
    }
}
