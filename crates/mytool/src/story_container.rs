use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::Button,
    dock::{Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    menu::PopupMenu,
    scroll::ScrollableElement,
};

use crate::{AppState, Mytool, ShowPanelInfo, StoryState};
#[derive(Debug)]
#[allow(dead_code)]
pub enum ContainerEvent {
    Close,
}
pub struct StoryContainer {
    pub(crate) focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<Pixels>,
    height: Option<Pixels>,
    story: Option<AnyView>,
    story_klass: Option<SharedString>,
    pub(crate) closable: bool,
    pub(crate) zoomable: Option<PanelControl>,
    paddings: Pixels,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
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
            story: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            paddings: px(16.),
            on_active: None,
        }
    }

    pub fn panel<S: Mytool>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(window, cx);
        let story_klass = S::klass();

        cx.new(|cx| {
            let mut story =
                Self::new(window, cx).story(story.into(), story_klass).on_active(S::on_active_any);
            story.focus_handle = cx.focus_handle();
            story.closable = S::closable();
            story.zoomable = S::zoomable();
            story.name = name.into();
            story.description = description.into();
            story.title_bg = S::title_bg();
            story.paddings = S::paddings();
            story
        })
    }

    #[allow(unused)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    #[allow(unused)]
    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn story(mut self, story: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.story = Some(story);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }
}
impl Panel for StoryContainer {
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.name.clone().into_any_element()
    }

    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        self.title_bg.map(|bg| TitleStyle { background: bg, foreground: cx.theme().foreground })
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx).invisible_panels.read(cx).contains(&self.name)
    }

    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut Context<Self>) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active
            && let Some(story) = self.story.clone()
        {
            on_active(story, active, _window, cx);
        }
    }

    fn dropdown_menu(
        &mut self,
        menu: PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    fn toolbar_buttons(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info").icon(IconName::Info).on_click(|_, _window, cx| {
                cx.dispatch_action(&crate::ShowPanelInfo);
            }),
            Button::new("search").icon(IconName::Search).on_click(|_, _window, cx| {
                cx.dispatch_action(&crate::ToggleSearch);
            }),
        ])
    }

    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        let klass = self.story_klass.clone().unwrap_or_else(|| SharedString::from("ListStory"));
        let story_state = StoryState { story_klass: klass };
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
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("story-container")
            .size_full()
            .overflow_y_scrollbar()
            .track_focus(&self.focus_handle)
            .when_some(self.story.clone(), |this, story| {
                this.child(div().size_full().p(self.paddings).child(story))
            })
    }
}
